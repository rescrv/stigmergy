use std::io::{self, Read};
use std::process;

use stigmergy::ComponentDefinition;

fn main() {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Failed to read stdin: {}", e);
        process::exit(1);
    }

    let definition: ComponentDefinition = match serde_json::from_str(&input) {
        Ok(def) => def,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            process::exit(1);
        }
    };

    match serde_yml::to_string(&definition) {
        Ok(yaml) => print!("{}", yaml),
        Err(e) => {
            eprintln!("Failed to serialize to YAML: {}", e);
            process::exit(1);
        }
    }
}
