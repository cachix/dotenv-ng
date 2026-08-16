# Changelog

## [Unreleased]

### Added

- Added minimal-quoting `render_value`, `render_var`, and `render` APIs for losslessly rendering
  values and assignments in the supported dotenv syntax.

### Changed and fixed

- Removed the runtime `#[load]` attribute because a procedural macro cannot prove that a function
  named `main` is the program entry point. Environment mutation now always requires an explicit
  unsafe `EnvLoader::load_and_modify` call before other threads start.
- Matched Windows' case-insensitive, case-preserving environment-name semantics during parsing,
  substitution, precedence resolution, map access, and merging.
- Tracked all process environment variables consulted by compile-time interpolation so changes
  invalidate Cargo's incremental build cache.
- Preserved escaped trailing spaces and tabs in unquoted values.
- Made `EnvLoader::default()` equivalent to `EnvLoader::new()`.
- Expanded parser, loader, macro, CLI, async-runtime, and packaging fixture coverage.

## [1.0.0]

This is the first `dotenv-ng` release. It starts from `dotenvy` 0.15.7, released
on March 22, 2023. Earlier release history belongs to `dotenvy` and is not
duplicated here.

Applications should normally consume the new package under the familiar crate
name:

```toml
[dependencies]
dotenv = { package = "dotenv-ng", version = "1" }
```

Enable the `macros` feature to use `dotenv!` or `option_dotenv!`.

This release uses Rust 2021 and has a minimum supported Rust version of 1.74.0.
The complete workspace, including the CLI and async-runtime examples, is tested
against that compiler version.

### Breaking changes

#### Packages and crate names

- The public package is now `dotenv-ng`; imports should use the dependency alias
  `dotenv` instead of `dotenvy`.
- The command-line executable is now `dotenv-ng` instead of `dotenvy`.
- The previous compile-time macro crate is replaced by the internal
  `dotenv-ng-macros` crate. Its macros are re-exported by `dotenv-ng` through
  the `macros` feature.
- Shared parsing and loading code now lives in the internal `dotenv-ng-core`
  crate. Applications should continue to depend only on `dotenv-ng`.

#### Parsing and loading

- Variable substitution is disabled by default. Dollar signs are preserved
  literally unless interpolation is enabled with `EnvLoader::substitution(true)`,
  `substitution = true` in a macro, or `--substitute` in the CLI.
- The parser has been replaced with a source-aware implementation. It accepts a
  broader set of environment names, unquoted spaces and structured values, and
  preserves unknown backslash escapes. Code relying on the old parser rejecting
  these inputs may behave differently.
- A `#` starts a comment only at the beginning of a line or after whitespace
  outside quotes. A value such as `value#literal` retains the hash.
- `Error::LineParse` is replaced by `Error::Parse(ParseError, Option<PathBuf>)`.
  `Error` and `ParseErrorKind` are non-exhaustive, and parse diagnostics now
  include the source line, Unicode column, byte offset, error kind, and optional
  file path.
- Invalid non-Unicode process environment names and values now return
  `Error::NotUnicodeName` and `Error::NotUnicode`, respectively, instead of
  panicking while the environment is read.
- `load_and_modify` parses and validates every input before changing the process
  environment. A failure no longer leaves a partially applied environment.

#### Macros

- `dotenv!` no longer loads values by mutating the compiler process environment.
  It parses the selected file into an isolated map and embeds the resulting
  value directly.
- Relative paths passed to `dotenv!` are resolved from the invoking package's
  `CARGO_MANIFEST_DIR`, not Cargo's working directory.
- The default compile-time `.env` file is optional so build-environment values
  can be used without a file. Parse errors and non-`NotFound` I/O errors remain
  compilation errors.
- Compile-time env files are tracked as compiler inputs, so changing a file
  triggers recompilation.
- The runtime `#[load]` attribute is removed. Callers must explicitly acknowledge
  `EnvLoader::load_and_modify` as unsafe and run it before starting other threads.

#### Command-line interface

- `--required` is replaced by `--optional`; files are required by default.
- `--file` may be repeated. Files are parsed in order and later files override
  earlier files.
- The CLI builds the child command's environment without modifying its own
  process environment.
- A single quoted command argument is split using shell-word rules. Shell
  operators such as pipes and redirections are not evaluated.
- Failures to execute a command use exit status 127 when the command is not
  found and 126 for other execution failures. Omitting the command is handled by
  Clap and exits with status 2.

### Added

- `EnvLoader::with_paths` and `EnvLoader::with_readers` for ordered, layered
  inputs. Later inputs win and can reference values from earlier inputs when
  substitution is enabled.
- `EnvLoader::required(false)` for skipping missing path inputs while preserving
  all other I/O and parse errors.
- `ParseOptions`, `EnvLoader::parse_options`, `EnvLoader::substitution`, and
  `EnvLoader::multiline` for explicit parser configuration.
- `EnvLoader::substitutions` for providing interpolation values without changing
  the process environment.
- Shell- and Compose-style interpolation operators, including `${VAR:-default}`,
  `${VAR-default}`, `${VAR:?message}`, `${VAR?message}`, `${VAR:+alternate}`,
  and `${VAR+alternate}`.
- `option_dotenv!`, which expands to `Option<&'static str>` when a compile-time
  value may be absent.
- `path`, `var`, `error`, `sequence`, `override_`, and `substitution` options for
  `dotenv!`.
- `--substitute` for opt-in CLI interpolation and quoted-command support.

### Changed and fixed

- Input precedence and interpolation lookup now consistently follow
  `EnvSequence`.
- `InputOnly` no longer inspects the process environment, including non-Unicode
  values.
- Multiple inputs share interpolation state, while explicit substitution values
  remain separate from the returned environment map.
- UTF-8 byte-order marks are stripped for every loading entry point.
- Quoted multiline values normalize CRLF input, and multiline parsing can be
  disabled explicitly.
- Unbraced substitutions include underscores in variable names.
- Environment names may contain leading digits, dashes, dots, Unicode, and other
  non-whitespace characters supported by the operating system.
- NUL characters in names and values are rejected before environment mutation.
- Windows paths, JSON, embedded quotes, and other structured unquoted values no
  longer require unnecessary escaping.
- `Error` now implements `From<std::io::Error>` and preserves path context for
  both I/O and parse failures.
- The CLI accepts non-Unicode command arguments where the operating system does.

### Upstream reports addressed

This release incorporates or supersedes the following work identified while
auditing `dotenvy` issues and pull requests:

- Unquoted spaces, JSON values, and Windows paths:
  [#11](https://github.com/allan2/dotenvy/issues/11),
  [#82](https://github.com/allan2/dotenvy/issues/82), and
  [#84](https://github.com/allan2/dotenvy/issues/84).
- Correct substitution precedence, caller-provided substitution values, and
  underscore handling:
  [#149](https://github.com/allan2/dotenvy/issues/149),
  [#150](https://github.com/allan2/dotenvy/issues/150), and
  [#170](https://github.com/allan2/dotenvy/issues/170).
- Literal dollar signs by default, with explicit opt-in interpolation for
  bcrypt hashes, passwords, tokens, and other secret values:
  [dotenvy #113](https://github.com/allan2/dotenvy/issues/113),
  [dotenvy PR #167](https://github.com/allan2/dotenvy/pull/167), and
  [Secretspec #73](https://github.com/cachix/secretspec/issues/73).
- UTF-8 BOM handling, deterministic tests, atomic loading, and surfaced parse
  failures:
  [#127](https://github.com/allan2/dotenvy/issues/127),
  [#163](https://github.com/allan2/dotenvy/issues/163), and
  [#168](https://github.com/allan2/dotenvy/issues/168).
- Manifest-relative and configurable compile-time env files:
  [#74](https://github.com/allan2/dotenvy/issues/74),
  [#128](https://github.com/allan2/dotenvy/issues/128), and
  [PR #159](https://github.com/allan2/dotenvy/pull/159).
- Multiple CLI env files and quoted commands:
  [#153](https://github.com/allan2/dotenvy/issues/153) and
  [#126](https://github.com/allan2/dotenvy/issues/126).
