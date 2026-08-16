# dotenv-ng

[![Crates.io](https://img.shields.io/crates/v/dotenv-ng.svg)](https://crates.io/crates/dotenv-ng)
[![msrv
1.74.0](https://img.shields.io/badge/msrv-1.74.0-dea584.svg?logo=rust)](https://github.com/rust-lang/rust/releases/tag/1.74.0)
[![docs](https://img.shields.io/docsrs/dotenv-ng?logo=docs.rs)](https://docs.rs/dotenv-ng/)
[![CI](https://github.com/cachix/dotenv-ng/actions/workflows/ci.yml/badge.svg)](https://github.com/cachix/dotenv-ng/actions/workflows/ci.yml)

A modernized fork of `dotenvy` for loading environment variables from `.env` files.

This project is maintained for [secretspec.dev](https://secretspec.dev/).

## Version 1.0

`dotenv-ng` 1.0 is a breaking fork release based on `dotenvy` 0.15.7.

## Components

1. [`dotenv-ng`](https://crates.io/crates/dotenv-ng) - The public library and CLI.
2. [`dotenv-ng-macros`](https://crates.io/crates/dotenv-ng-macros) - The implementation of the compile-time `dotenv!` and `option_dotenv!` macros.

Applications should normally depend only on `dotenv-ng`, aliased to the familiar crate name `dotenv`. Enable its `macros` feature to use the compile-time macros.

```toml
[dependencies]
dotenv = { package = "dotenv-ng", version = "1", features = ["macros"] }
```

## What is an environment file?

An _environment file_, or _env file_, is a plain text file consisting of key-value pairs.

_.env_

```sh
HOST=foo
PORT=3000
```

Common names for env files are _.env_, _.env.dev_, _.env.prod_, but any name can be used. The default path for this crate is _.env_.

### Supported syntax

- Keys are separated from values by `=`. Names may contain digits, dashes, dots, Unicode, and other non-whitespace characters supported by the operating system.
- Unquoted values may contain spaces and structured text such as JSON. Leading and trailing whitespace is ignored.
- Single- and double-quoted values may span lines. Single-quoted values preserve content except for `\\` and `\'`; double-quoted values support common escapes such as `\n`, `\r`, `\t`, `\\`, and `\"`.
- Unknown backslash escapes are preserved, so unquoted Windows paths such as `C:\Users\me` work as written.
- A `#` begins a comment at the start of a line or after whitespace outside quotes. `value#literal` retains the hash.
- An optional `export` prefix is accepted.
- Dollar signs are literal by default. With substitution enabled, `$NAME` and `${NAME}` expand variables; Shell/Compose-style forms such as `${NAME:-default}` are supported.

For example:

```sh
HOST=example.com
URL=https://${HOST}/api
TIMEOUT=${TIMEOUT:-30}
REQUIRED=${REQUIRED:?set REQUIRED before loading this file}
```

## Usage

At runtime, this library supports non-environment-modifying and environment-modifying workflows.
The optional `macros` feature also supports compile-time loading.

## Runtime loading

The non-modifying API is recommended for most use cases.

### Non-modifying API

```rs
use dotenv::EnvLoader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_map = EnvLoader::new().load()?;
    println!("HOST={}", env_map.var("HOST")?);
    Ok(())
}
```

#### Configuration

```rs
use dotenv::{EnvLoader, EnvSequence};
use std::io::Cursor;

// from a file
let loader1 = EnvLoader::with_path("./.env").sequence(EnvSequence::InputThenEnv);
let loader2 = EnvLoader::new();  // shorthand for loader1

// parse files in order; later files override earlier files
let layered_loader = EnvLoader::with_paths([".env", ".env.local"]);

// from a string
let s = "HOST=foo\nPORT=3000";
let str_loader = EnvLoader::with_reader(Cursor::new(s));

// let file values win over inherited environment values in the returned map
let overriding_loader = EnvLoader::new().sequence(EnvSequence::EnvThenInput);

// opt in to variable interpolation
let interpolating_loader = EnvLoader::new().substitution(true);

// reject multiline quoted values (dollar signs are already literal by default)
let strict_loader = EnvLoader::new()
    .multiline(false);

// make application values available to substitution without changing the process environment
let injected_loader = EnvLoader::new()
    .substitution(true)
    .substitutions([("APP_NAME", "example")]);
```

Loader construction is infallible. When reading from a path, I/O is deferred until the `load` call.
This supports configurations such as dev/prod selection and optional loading.

`EnvOnly` returns the existing process environment without reading input. When substitution is
enabled, the other sequences also control interpolation precedence: `InputOnly` never reads the
process environment, `EnvThenInput` lets input assignments replace environment values, and
`InputThenEnv` keeps environment values authoritative. Parsing and validation complete before
`load_and_modify` changes any environment variable.

For multiple inputs, later files override earlier files and, with substitution enabled, can use
values defined by earlier files. `required(false)` skips missing paths while still reporting
malformed files and other I/O failures.

### Rendering

`render_value`, `render_var`, and `render` produce dotenv syntax that round-trips through the
default parser. Values remain unquoted whenever their original text already parses unchanged;
otherwise they are double-quoted and escaped.

```rs
use dotenv::{render, render_value, render_var};

assert_eq!(render_value("hello world")?.as_ref(), "hello world");
assert_eq!(render_var("GREETING", " hello ")?, "GREETING=\" hello \"");
assert_eq!(
    render([("HOST", "example.com"), ("PORT", "3000")])?,
    "HOST=example.com\nPORT=3000"
);
# Ok::<(), dotenv::RenderError>(())
```

Rendering follows `dotenv-ng` syntax rather than claiming compatibility with every dotenv
dialect. Assignment order follows the supplied iterator, and `render` does not add a trailing
newline.

### Modifying API

There are situations where modifying the environment is necessary.
For example, you may be spawning a child process that reads the environment.

Call `EnvLoader::load_and_modify` explicitly at program startup, before starting any additional
threads or an async runtime:

```rs
use dotenv::EnvLoader;

fn main() -> Result<(), dotenv::Error> {
    let loader = EnvLoader::new();
    // SAFETY: this is the first operation in `main`; no additional threads have started.
    unsafe { loader.load_and_modify()? };
    println!("HOST={}", std::env::var("HOST").unwrap());
    Ok(())
}
```

The call is unsafe because [`set_var`](https://doc.rust-lang.org/stable/std/env/fn.set_var.html)
cannot be made thread-safe by a library on Unix. For async applications, perform the load from a
synchronous `main` and construct the runtime afterward; see the Tokio example in this repository.

## Compile-time loading

With the `macros` feature enabled, `dotenv::dotenv!` reads a variable while compiling and expands to a string literal:

```rs
const HOST: &str = dotenv::dotenv!("HOST");
const TEST_HOST: &str = dotenv::dotenv!("HOST", path = "config/test.env");
const OPTIONAL_HOST: Option<&str> = dotenv::option_dotenv!("HOST");
```

Compile-time paths are relative to the package containing the macro call, not the workspace or
Cargo working directory. `path`, `sequence`, `override_`, and `substitution` are configurable.
Dollar signs are literal unless `substitution = true`. Existing env files and every environment
variable consulted during interpolation are tracked as compiler inputs, so changing them triggers
recompilation. Stable procedural-macro APIs cannot track creation of a previously missing optional
file; after creating one, touch the invoking Rust source or run `cargo clean -p <package>`. The value is embedded in the
binary; do not use this macro for secrets that should not be present in build artifacts.

## Command-line tool

Enable the `cli` feature when installing. Repeat `--file` to layer files; later files win:

```sh
cargo install dotenv-ng --features cli
dotenv-ng --file .env --file .env.local -- cargo run
```

Use `--override` to let files replace inherited variables, `--optional` to skip missing files,
and `--substitute` to opt in to variable interpolation.
A single quoted command such as `dotenv-ng "cargo run --release"` is split using Unix shell-word
rules. Shell operators such as pipes and redirections are not evaluated.

## Minimum Supported Rust Version

`dotenv-ng` 1.0 supports Rust 1.74 and newer. Increasing the MSRV is not considered a
semver-breaking change.

## Why does this fork exist?

`dotenv-ng` continues `dotenvy` under active maintenance for SecretSpec while allowing the
breaking changes needed for a coherent 1.0 API. The registry package has a distinct name, while
Cargo dependency aliasing preserves the familiar `dotenv` crate name in Rust code.

## What changed in 1.0?

- renamed the public package to `dotenv-ng`, normally consumed under the crate name `dotenv`
- consolidated the compile-time macros behind one `macros` feature
- added ordered multi-file and multi-reader loading
- added configurable substitution, multiline parsing, and structured parse errors
- made compile-time paths package-relative and tracked env files as compiler inputs
- stopped compile-time loading and the CLI from modifying their own process environments
- added optional compile-time values with `option_dotenv!`
- made process-environment mutation explicit and unsafe at the application entry point

See `CHANGELOG.md` in the repository for the complete list of changes.

## Contributing

Contributions are welcome. See `CONTRIBUTING.md` in the repository to get started.

`dotenv-ng` is derived from `dotenvy`, which in turn is derived from the original `dotenv` crate.
