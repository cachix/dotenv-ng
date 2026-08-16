const VALUE: &str = envfile::dotenv!("CODEGEN_TEST_VAR1", path = "../../.env");

fn main() {
    assert_eq!(VALUE, "hello!");
}
