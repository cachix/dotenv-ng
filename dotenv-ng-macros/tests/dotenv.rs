const MANIFEST_VALUE: &str = dotenv_ng_macros::dotenv!("MACRO_MANIFEST_VALUE");
const LITERAL_VALUE: &str = dotenv_ng_macros::dotenv!("MACRO_LITERAL");
const EXPANDED_VALUE: &str = dotenv_ng_macros::dotenv!("MACRO_LITERAL", substitution = true);

#[test]
fn dotenv_works() {
    assert_eq!(MANIFEST_VALUE, "macro-package");
    assert_eq!(LITERAL_VALUE, "$MACRO_SOURCE");
    assert_eq!(EXPANDED_VALUE, "expanded");
    assert_eq!(
        dotenv_ng_macros::dotenv!("CODEGEN_TEST_VAR1", path = "tests/fixtures/macro.env"),
        "hello!"
    );
}

#[test]
fn two_argument_form_works() {
    assert_eq!(
        dotenv_ng_macros::dotenv!(
            "CODEGEN_TEST_VAR2",
            "custom missing variable error",
            path = "tests/fixtures/macro.env"
        ),
        "'quotes within quotes'"
    );
}

#[test]
fn option_and_sequence_configuration_work() {
    assert_eq!(
        dotenv_ng_macros::option_dotenv!(
            "DEFINITELY_NOT_DEFINED_BY_DOTENV_NG",
            path = "missing.env",
            sequence = "input-only"
        ),
        None
    );
    assert_eq!(
        dotenv_ng_macros::option_dotenv!("MACRO_MANIFEST_VALUE"),
        Some("macro-package")
    );
}
