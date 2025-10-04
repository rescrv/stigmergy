use std::fs;
use std::process;

use arrrg::CommandLine;
use arrrg_derive::CommandLine;

use stigmergy::SystemParser;

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Options {
    #[arrrg(flag, "Enable verbose output showing pass/fail for each file")]
    verbose: bool,
}

fn main() {
    let (options, free) =
        Options::from_command_line("USAGE: validate-system [--verbose] <file>...");

    if free.is_empty() {
        process::exit(1);
    }

    let mut all_valid = true;

    for path in &free {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                if options.verbose {
                    println!("{} fail", path);
                }
                all_valid = false;
                continue;
            }
        };

        match SystemParser::parse(&content) {
            Ok(config) => {
                if config.validate().is_err() {
                    if options.verbose {
                        println!("{} fail", path);
                    }
                    all_valid = false;
                } else if options.verbose {
                    println!("{} pass", path);
                }
            }
            Err(_) => {
                if options.verbose {
                    println!("{} fail", path);
                }
                all_valid = false;
            }
        }
    }

    if all_valid {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
