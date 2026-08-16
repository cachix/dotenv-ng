#![deny(clippy::uninlined_format_args, clippy::wildcard_imports)]

//! Load environment variables from an env file or a reader.
//!
//! Enable the `macros` feature to use `dotenv!` at compile time or `load` at runtime.

pub use dotenv_ng_core::*;

#[cfg(feature = "macros")]
pub use dotenv_ng_macros::{dotenv, option_dotenv};
