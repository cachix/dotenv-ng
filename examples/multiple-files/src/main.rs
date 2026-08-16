use dotenv::{EnvLoader, EnvSequence};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    // Files are parsed in order. The second file can refer to values from the first, and its
    // assignments override the first file's assignments.
    let env_map = EnvLoader::with_paths(["../env-example", "../env-example-2"])
        .sequence(EnvSequence::EnvThenInput)
        .substitution(true)
        .load()?;

    if let Some(v) = env_map.get("HOST") {
        println!("HOST={v}");
    }
    Ok(())
}
