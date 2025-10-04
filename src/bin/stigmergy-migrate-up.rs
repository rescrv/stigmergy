//! Database migration tool for Stigmergy.
//!
//! This binary runs database migrations using sqlx's compile-time checked migrations.
//! Migrations are embedded into the binary at compile time from the `migrations/` directory.

use arrrg::CommandLine;
use arrrg_derive::CommandLine;

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Options {
    #[arrrg(required, "PostgreSQL database URL")]
    database_url: String,
}

const USAGE: &str = r#"Usage: stigmergy-migrate-up --database-url <URL>

Run database migrations for Stigmergy.

Arguments:
  --database-url <URL>    PostgreSQL database connection URL

Example:
  stigmergy-migrate-up --database-url postgres://user:pass@localhost/stigmergy

The migrations are embedded at compile time from the migrations/ directory."#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (options, free) = Options::from_command_line(USAGE);

    if !free.is_empty() {
        eprintln!("Error: Unexpected arguments: {:?}", free);
        eprintln!();
        eprintln!("{}", USAGE);
        std::process::exit(1);
    }

    println!("Connecting to database: {}", options.database_url);

    // Connect to PostgreSQL
    let pool = sqlx::PgPool::connect(&options.database_url).await?;

    println!("Running migrations...");

    // Run migrations embedded at compile time
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("Migrations completed successfully!");

    Ok(())
}
