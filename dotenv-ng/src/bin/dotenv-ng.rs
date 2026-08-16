//! Runs a command with variables loaded from one or more env files.
//!
//! Later files override earlier files:
//!
//! ```sh
//! dotenv-ng -f .env -f .env.local printenv HOST
//! ```
use clap::{ArgAction, Parser, Subcommand};
use dotenv::{EnvLoader, EnvSequence};
use std::{
    error,
    ffi::{OsStr, OsString},
    io,
    path::PathBuf,
    process,
};

#[derive(Parser)]
#[command(
    name = "dotenv-ng",
    version,
    about = "Run a command with variables loaded from env files",
    arg_required_else_help = true,
    allow_external_subcommands = true
)]
struct Cli {
    /// Path to an env file; may be repeated, with later files taking precedence
    #[arg(
        short,
        long = "file",
        value_name = "PATH",
        action = ArgAction::Append,
        default_value = "./.env"
    )]
    files: Vec<PathBuf>,

    /// Continue when an env file does not exist
    #[arg(long)]
    optional: bool,

    /// Let env-file values override variables inherited from this process
    #[arg(long)]
    r#override: bool,

    /// Expand `$NAME` and `${NAME}` references in env-file values
    #[arg(long)]
    substitute: bool,

    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();
    let sequence = if cli.r#override {
        EnvSequence::EnvThenInput
    } else {
        EnvSequence::InputThenEnv
    };

    // A child-specific environment avoids unsafe mutation of this multi-thread-capable process.
    // Without substitution, parsing does not need a Unicode copy of the inherited environment.
    // Let Command preserve the raw OS environment and apply only the file-derived overlay. This
    // keeps unrelated non-Unicode Unix variables from preventing ordinary CLI use.
    let loader_sequence = if cli.substitute {
        sequence
    } else {
        EnvSequence::InputOnly
    };
    let mut environment = EnvLoader::with_paths(&cli.files)
        .required(!cli.optional)
        .sequence(loader_sequence)
        .substitution(cli.substitute)
        .load()?;
    if !cli.substitute && !cli.r#override {
        environment.retain(|key, _| std::env::var_os(key).is_none());
    }

    let Command::External(arguments) = cli.subcommand;
    let arguments = split_quoted_command(arguments)?;
    let (program, arguments) = arguments.split_first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "the command cannot be empty")
    })?;
    let mut command = make_command(program, arguments);
    command.envs(environment);

    run(command, program)
}

fn split_quoted_command(arguments: Vec<OsString>) -> io::Result<Vec<OsString>> {
    let [command] = arguments.as_slice() else {
        return Ok(arguments);
    };
    let Some(command) = command.to_str() else {
        return Ok(arguments);
    };
    shell_words::split(command)
        .map(|parts| parts.into_iter().map(OsString::from).collect())
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid quoted command: {error}"),
            )
        })
}

fn make_command(program: &OsStr, arguments: &[OsString]) -> process::Command {
    let mut command = process::Command::new(program);
    command.args(arguments);
    command
}

fn run(mut command: process::Command, program: &OsStr) -> Result<(), Box<dyn error::Error>> {
    #[cfg(windows)]
    {
        match command.spawn().and_then(|mut child| child.wait()) {
            Ok(status) => process::exit(status.code().unwrap_or(1)),
            Err(error) => {
                eprintln!("dotenv-ng: failed to execute {program:?}: {error}");
                process::exit(if error.kind() == io::ErrorKind::NotFound {
                    127
                } else {
                    126
                });
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let error = command.exec();
        eprintln!("dotenv-ng: failed to execute {program:?}: {error}");
        process::exit(if error.kind() == io::ErrorKind::NotFound {
            127
        } else {
            126
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        let status = command.status()?;
        process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::{split_quoted_command, Cli};
    use clap::Parser;
    use std::{ffi::OsString, path::PathBuf};

    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn accepts_multiple_files_in_order() {
        let cli = Cli::try_parse_from([
            "dotenv-ng",
            "--file",
            "base.env",
            "--file",
            "local.env",
            "printenv",
            "HOST",
        ])
        .unwrap();
        assert_eq!(
            cli.files,
            [PathBuf::from("base.env"), PathBuf::from("local.env")]
        );
    }

    #[test]
    fn splits_a_single_quoted_command_argument() {
        let parts =
            split_quoted_command(vec![OsString::from("printf '%s\\n' 'hello world'")]).unwrap();
        assert_eq!(
            parts,
            [
                OsString::from("printf"),
                OsString::from("%s\\n"),
                OsString::from("hello world")
            ]
        );
    }

    #[test]
    fn rejects_an_unterminated_quoted_command() {
        let error = split_quoted_command(vec![OsString::from("printf 'unterminated")]).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_a_non_unicode_program_name() {
        let program = OsString::from_vec(vec![b'p', 0xff]);
        assert_eq!(
            split_quoted_command(vec![program.clone()]).unwrap(),
            [program]
        );
    }
}
