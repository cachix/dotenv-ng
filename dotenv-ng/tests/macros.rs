#![cfg(feature = "macros")]

const CODEGEN_VALUE: &str = dotenv::dotenv!("CODEGEN_TEST_VAR1", path = "tests/fixtures/macro.env");
const MANIFEST_VALUE: &str = dotenv::dotenv!("FACADE_MANIFEST_VALUE");

#[test]
fn compile_time_macro_is_reexported() {
    assert_eq!(CODEGEN_VALUE, "hello!");
    assert_eq!(MANIFEST_VALUE, "facade-package");
    assert_eq!(
        dotenv::option_dotenv!(
            "DEFINITELY_NOT_DEFINED_BY_DOTENV_NG",
            sequence = "input-only"
        ),
        None
    );
}
