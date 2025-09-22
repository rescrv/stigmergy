use std::process;

/// Exits the program with an error message
pub fn exit_with_error(message: &str) -> ! {
    eprintln!("Error: {}", message);
    process::exit(1);
}

/// Exits the program with an error message and usage information
pub fn exit_with_usage_error(message: &str, usage: &str) -> ! {
    eprintln!("Error: {}", message);
    eprintln!("{}", usage);
    process::exit(1);
}

/// Prints a formatted success message
pub fn print_success(message: &str) {
    println!("{}", message);
}

/// Prints formatted JSON with proper indentation
pub fn print_json<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: serde::Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Prints a formatted JSON value or exits with error
pub fn print_json_or_exit<T>(value: &T, context: &str)
where
    T: serde::Serialize,
{
    if let Err(e) = print_json(value) {
        exit_with_error(&format!("Failed to format {} JSON: {}", context, e));
    }
}
