# dotenv-ng-macros

Procedural macro implementation for `dotenv-ng`.

This crate provides the compile-time `dotenv!` and `option_dotenv!` macros. Applications should
normally depend on `dotenv-ng` under the name `dotenv`, enable its `macros` feature, and use the
re-exported APIs.
