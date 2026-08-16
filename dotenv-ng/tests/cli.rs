#![cfg(feature = "cli")]

use std::{
    env,
    path::PathBuf,
    process::{self, Command},
};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

fn missing_env_file(test_name: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "dotenv-ng-{test_name}-{}-missing.env",
        process::id()
    ))
}

#[test]
fn optional_missing_file_runs_the_command() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("optional");
    let status = Command::new(executable)
        .args([
            "--file",
            missing_file.to_str().unwrap(),
            "--optional",
            executable,
            "--version",
        ])
        .output()
        .unwrap();

    assert!(status.status.success());
}

#[cfg(unix)]
#[test]
fn unrelated_non_unicode_environment_values_are_preserved() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("non-unicode-environment");
    let output = Command::new(executable)
        .arg("--file")
        .arg(missing_file)
        .arg("--optional")
        .args([executable, "--version"])
        .env("DOTENV_NG_NON_UNICODE_CLI", OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("dotenv-ng"));
}

#[test]
fn missing_file_is_required_by_default() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("required");
    let status = Command::new(executable)
        .args([
            "--file",
            missing_file.to_str().unwrap(),
            executable,
            "--version",
        ])
        .output()
        .unwrap();

    assert!(!status.status.success());
}

#[test]
fn a_single_quoted_command_is_split() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("quoted");
    let command = format!("{} --version", shell_words::quote(executable));
    let output = Command::new(executable)
        .args([
            "--file",
            missing_file.to_str().unwrap(),
            "--optional",
            &command,
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("dotenv-ng"));
}

#[cfg(unix)]
#[test]
fn multiple_files_are_loaded_in_order() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let output = Command::new(executable)
        .arg("--file")
        .arg(fixtures.join("cli-base.env"))
        .arg("--file")
        .arg(fixtures.join("cli-local.env"))
        .arg("--substitute")
        .args(["sh", "-c", "printf '%s' \"$CLI_DERIVED|$CLI_SHARED\""])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "base-local|local");
}

#[cfg(unix)]
#[test]
fn dollar_signs_are_literal_by_default() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let output = Command::new(executable)
        .arg("--file")
        .arg(fixtures.join("cli-literal.env"))
        .args(["sh", "-c", "printf '%s' \"$CLI_SECRET\""])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "foo:$2a$10$TWoviNHS27HJMw1PKe4tBeIMlms6tWdYS9hKoHANKCQhluDlEt/gu,\
         bar:$2a$10$labXlt9fBRMjJu.gOUabjebLVBKGB/xZOFpEn/esCln56USXHMHQW"
    );
}

#[cfg(unix)]
#[test]
fn override_flag_controls_inherited_environment_precedence() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli-base.env");

    let inherited = Command::new(executable)
        .arg("--file")
        .arg(&fixture)
        .args(["sh", "-c", "printf '%s' \"$CLI_SHARED\""])
        .env("CLI_SHARED", "parent")
        .output()
        .unwrap();
    assert!(inherited.status.success());
    assert_eq!(String::from_utf8_lossy(&inherited.stdout), "parent");

    let overridden = Command::new(executable)
        .arg("--file")
        .arg(fixture)
        .arg("--override")
        .args(["sh", "-c", "printf '%s' \"$CLI_SHARED\""])
        .env("CLI_SHARED", "parent")
        .output()
        .unwrap();
    assert!(overridden.status.success());
    assert_eq!(String::from_utf8_lossy(&overridden.stdout), "base");
}

#[cfg(windows)]
#[test]
fn substitution_and_precedence_use_case_insensitive_environment_names() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli-windows-case.env");

    let run = |override_: bool| {
        let mut command = Command::new(executable);
        command.arg("--file").arg(&fixture).arg("--substitute");
        if override_ {
            command.arg("--override");
        }
        command
            .args([
                "cmd.exe",
                "/D",
                "/C",
                "<nul set /p =%DOTENV_NG_WINDOWS_CASE%^|%DOTENV_NG_WINDOWS_DERIVED%&exit /b 0",
            ])
            .env("Dotenv_Ng_Windows_Case", "parent")
            .output()
            .unwrap()
    };

    let inherited = run(false);
    assert!(inherited.status.success());
    assert_eq!(String::from_utf8_lossy(&inherited.stdout), "parent|parent");

    let overridden = run(true);
    assert!(overridden.status.success());
    assert_eq!(String::from_utf8_lossy(&overridden.stdout), "file|file");
}

#[cfg(unix)]
#[test]
fn child_exit_status_is_preserved() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("exit-status");
    let status = Command::new(executable)
        .arg("--file")
        .arg(missing_file)
        .arg("--optional")
        .args(["sh", "-c", "exit 23"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(23));
}

#[test]
fn missing_command_uses_shell_not_found_status() {
    let executable = env!("CARGO_BIN_EXE_dotenv-ng");
    let missing_file = missing_env_file("missing-command");
    let status = Command::new(executable)
        .arg("--file")
        .arg(missing_file)
        .arg("--optional")
        .arg("dotenv-ng-command-that-cannot-exist")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(127));
}
