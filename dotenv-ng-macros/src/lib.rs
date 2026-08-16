#![deny(clippy::uninlined_format_args, clippy::wildcard_imports)]

use dotenv_ng_core::{EnvLoader, EnvSequence};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::{
    collections::{BTreeSet, HashSet},
    env,
    path::{Path, PathBuf},
};
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitBool, LitStr, Token,
};

/// Reads an environment variable at compile time and expands to a string literal.
///
/// Relative paths start at the invoking package's `CARGO_MANIFEST_DIR`, never Cargo's current
/// working directory. The default path is `.env`. A missing file is allowed so that build
/// environment variables can be used without an env file. Dollar signs are literal unless
/// `substitution = true` is supplied.
///
/// ```ignore
/// const HOST: &str = dotenv!("HOST");
/// const TEST_HOST: &str = dotenv!("HOST", path = "config/test.env");
/// const FILE_ONLY: &str = dotenv!("HOST", sequence = "input-only");
/// const EXPANDED: &str = dotenv!("HOST", substitution = true);
/// ```
///
/// An optional second positional string, or `error = "..."`, customizes the diagnostic emitted
/// when the variable is absent.
#[proc_macro]
pub fn dotenv(input: TokenStream) -> TokenStream {
    expand_dotenv(input.into(), false).into()
}

/// Reads an optional environment variable at compile time.
///
/// This accepts the same `path`, `sequence`, `override_`, and `substitution` configuration as
/// [`dotenv!`]. It expands to `None` when neither the selected env file nor the build environment
/// defines the variable. Parse and non-`NotFound` I/O errors remain compile errors.
#[proc_macro]
pub fn option_dotenv(input: TokenStream) -> TokenStream {
    expand_dotenv(input.into(), true).into()
}

fn expand_dotenv(input: TokenStream2, optional: bool) -> TokenStream2 {
    match expand_dotenv_result(input, optional) {
        Ok(expanded) => expanded,
        Err(error) => error.into_compile_error(),
    }
}

fn expand_dotenv_result(input: TokenStream2, optional: bool) -> syn::Result<TokenStream2> {
    let args = syn::parse2::<DotenvInput>(input)?;
    if let (true, Some(error)) = (optional, args.error.as_ref()) {
        return Err(syn::Error::new(
            error.span(),
            "option_dotenv! does not accept an error message",
        ));
    }

    let path = resolve_manifest_path(args.path.as_ref())?;
    let (map, substitution_dependencies) = EnvLoader::with_path(&path)
        .required(false)
        .sequence(args.sequence)
        .substitution(args.substitution)
        .load_with_substitution_dependencies()
        .map_err(|error| syn::Error::new(args.path_span(), error.to_string()))?;
    let value = map.get(&args.variable.value());

    if !optional && value.is_none() {
        let message = args.error.map_or_else(
            || {
                format!(
                    "environment variable `{}` is not defined in `{}` or the build environment",
                    args.variable.value(),
                    path.display()
                )
            },
            |literal| literal.value(),
        );
        return Err(syn::Error::new(args.variable.span(), message));
    }

    tracked_expression(
        value,
        optional,
        &args.variable,
        &path,
        &substitution_dependencies,
    )
}

fn tracked_expression(
    value: Option<&String>,
    optional: bool,
    variable: &LitStr,
    path: &Path,
    substitution_dependencies: &HashSet<String>,
) -> syn::Result<TokenStream2> {
    let track_file = if path.is_file() {
        let path = path.to_str().ok_or_else(|| {
            syn::Error::new(
                variable.span(),
                "the resolved env path is not valid Unicode and cannot be tracked",
            )
        })?;
        let path = LitStr::new(path, Span::call_site());
        quote!(
            const _: &str = include_str!(#path);
        )
    } else {
        TokenStream2::new()
    };
    let mut tracked_variables = BTreeSet::from([variable.value()]);
    tracked_variables.extend(substitution_dependencies.iter().cloned());
    let tracked_variables: Vec<_> = tracked_variables
        .into_iter()
        .map(|name| LitStr::new(&name, Span::call_site()))
        .collect();
    let result = match (optional, value) {
        (true, Some(value)) => quote!(Some(#value)),
        (true, None) => quote!(None::<&'static str>),
        (false, Some(value)) => quote!(#value),
        (false, None) => unreachable!("required values are checked before expansion"),
    };

    Ok(quote!({
        #track_file
        #(const _: Option<&str> = option_env!(#tracked_variables);)*
        #result
    }))
}

fn resolve_manifest_path(path: Option<&LitStr>) -> syn::Result<PathBuf> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
        syn::Error::new(
            path.map_or_else(Span::call_site, LitStr::span),
            "CARGO_MANIFEST_DIR is unavailable while expanding the macro",
        )
    })?;
    let configured = path.map_or_else(|| PathBuf::from(".env"), |path| path.value().into());
    if configured.is_absolute() {
        Ok(configured)
    } else {
        Ok(PathBuf::from(manifest_dir).join(configured))
    }
}

struct DotenvInput {
    variable: LitStr,
    path: Option<LitStr>,
    error: Option<LitStr>,
    sequence: EnvSequence,
    substitution: bool,
}

impl DotenvInput {
    fn path_span(&self) -> Span {
        self.path
            .as_ref()
            .map_or_else(|| self.variable.span(), LitStr::span)
    }
}

impl Parse for DotenvInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut positional = Vec::new();
        let mut variable = None;
        let mut path = None;
        let mut error = None;
        let mut sequence = None;
        let mut override_ = None;
        let mut substitution = None;

        while !input.is_empty() {
            if input.peek(LitStr) {
                positional.push(input.parse::<LitStr>()?);
            } else {
                let ident = input.parse::<Ident>()?;
                input.parse::<Token![=]>()?;
                match ident.to_string().as_str() {
                    "var" => set_once(&mut variable, input.parse::<LitStr>()?, &ident)?,
                    "path" => set_once(&mut path, input.parse::<LitStr>()?, &ident)?,
                    "error" => set_once(&mut error, input.parse::<LitStr>()?, &ident)?,
                    "sequence" => {
                        let value = input.parse::<LitStr>()?;
                        let parsed = parse_sequence(&value)?;
                        set_once(&mut sequence, parsed, &ident)?;
                    }
                    "override_" => {
                        let value = input.parse::<LitBool>()?.value;
                        set_once(&mut override_, value, &ident)?;
                    }
                    "substitution" => {
                        let value = input.parse::<LitBool>()?.value;
                        set_once(&mut substitution, value, &ident)?;
                    }
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("unknown dotenv! option `{ident}`"),
                        ));
                    }
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        if positional.len() > 2 {
            return Err(syn::Error::new(
                positional[2].span(),
                "dotenv! accepts at most two positional arguments",
            ));
        }
        if let Some(positional_variable) = positional.first() {
            if variable.is_some() {
                return Err(syn::Error::new(
                    positional_variable.span(),
                    "the variable was supplied both positionally and with `var`",
                ));
            }
            variable = Some(positional_variable.clone());
        }
        if let Some(positional_error) = positional.get(1) {
            if error.is_some() {
                return Err(syn::Error::new(
                    positional_error.span(),
                    "the error message was supplied twice",
                ));
            }
            error = Some(positional_error.clone());
        }
        if sequence.is_some() && override_.is_some() {
            return Err(syn::Error::new(
                input.span(),
                "`sequence` and `override_` cannot be used together",
            ));
        }

        let sequence = sequence.unwrap_or(match override_ {
            Some(true) => EnvSequence::EnvThenInput,
            Some(false) | None => EnvSequence::InputThenEnv,
        });
        let variable = variable.ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "dotenv! requires an environment variable name",
            )
        })?;

        Ok(Self {
            variable,
            path,
            error,
            sequence,
            substitution: substitution.unwrap_or(false),
        })
    }
}

fn parse_sequence(value: &LitStr) -> syn::Result<EnvSequence> {
    match value.value().as_str() {
        "env-only" | "env_only" => Ok(EnvSequence::EnvOnly),
        "env-then-input" | "env_then_input" => Ok(EnvSequence::EnvThenInput),
        "input-only" | "input_only" => Ok(EnvSequence::InputOnly),
        "input-then-env" | "input_then_env" => Ok(EnvSequence::InputThenEnv),
        _ => Err(syn::Error::new(
            value.span(),
            "sequence must be `env-only`, `env-then-input`, `input-only`, or `input-then-env`",
        )),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, ident: &Ident) -> syn::Result<()> {
    if slot.replace(value).is_some() {
        Err(syn::Error::new(
            ident.span(),
            format!("`{ident}` can only be set once"),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_sequence, tracked_expression, DotenvInput};
    use proc_macro2::Span;
    use quote::quote;
    use std::{collections::HashSet, path::Path};
    use syn::{parse2, LitStr};

    #[test]
    fn dotenv_input_accepts_named_and_legacy_options() {
        let input: DotenvInput = parse2(quote!(
            var = "HOST",
            path = "config.env",
            override_ = true,
            substitution = true,
            error = "missing"
        ))
        .unwrap();
        assert_eq!(input.variable.value(), "HOST");
        assert_eq!(input.path.unwrap().value(), "config.env");
        assert!(input.substitution);
        assert_eq!(input.error.unwrap().value(), "missing");
    }

    #[test]
    fn dotenv_input_rejects_conflicting_and_duplicate_options() {
        let conflict =
            parse2::<DotenvInput>(quote!("HOST", sequence = "input-only", override_ = true))
                .err()
                .expect("conflicting options should be rejected");
        assert!(conflict.to_string().contains("cannot be used together"));

        let duplicate = parse2::<DotenvInput>(quote!(var = "HOST", var = "PORT"))
            .err()
            .expect("duplicate options should be rejected");
        assert!(duplicate.to_string().contains("can only be set once"));
    }

    #[test]
    fn tracked_expression_tracks_requested_and_interpolated_variables() {
        let variable = LitStr::new("RESULT", Span::call_site());
        let dependencies = HashSet::from(["SOURCE".to_owned(), "FALLBACK".to_owned()]);
        let value = "value".to_owned();
        let expanded = tracked_expression(
            Some(&value),
            false,
            &variable,
            Path::new("definitely-missing.env"),
            &dependencies,
        )
        .unwrap()
        .to_string();

        assert!(expanded.contains("RESULT"));
        assert!(expanded.contains("SOURCE"));
        assert!(expanded.contains("FALLBACK"));
    }

    #[test]
    fn sequence_parser_accepts_hyphenated_and_underscored_spellings() {
        for spelling in [
            "input-only",
            "input_only",
            "env-then-input",
            "env_then_input",
        ] {
            let literal = LitStr::new(spelling, Span::call_site());
            parse_sequence(&literal).unwrap();
        }
    }
}
