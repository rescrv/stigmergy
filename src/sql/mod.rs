//! PostgreSQL database operations for stigmergy.
//!
//! This module provides functions for interacting with the PostgreSQL database,
//! organized by data type.

/// Entity operations with automatic timestamp tracking.
pub mod entity;

/// Component definition operations with automatic timestamp tracking.
pub mod component_definition;

#[cfg(test)]
/// Test utilities for PostgreSQL database operations.
pub mod tests {
    use sqlx::PgPool;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Creates a unique test database for each test invocation.
    ///
    /// This function creates a new PostgreSQL database with a unique name based on
    /// the process ID, current timestamp, and an atomic counter. The database is fully
    /// isolated from other tests and should be cleaned up after the test completes.
    ///
    /// The database is created by:
    /// 1. Connecting to the base database URL
    /// 2. Creating a new database with a unique name
    /// 3. Running migrations on the new database
    /// 4. Returning a connection pool to the new database
    pub async fn setup_test_db() -> PgPool {
        let base_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/stigmergy_test".to_string());

        let pid = std::process::id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("stigmergy_test_{}_{}_{}", pid, timestamp, counter);

        let mut parsed_url = url::Url::parse(&base_url).expect("Invalid database URL");

        let admin_pool = PgPool::connect(&base_url)
            .await
            .expect("Failed to connect to test database");

        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("Failed to create test database");

        admin_pool.close().await;

        parsed_url.set_path(&format!("/{}", db_name));
        let test_db_url = parsed_url.as_str();

        let pool = PgPool::connect(test_db_url)
            .await
            .expect("Failed to connect to test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }
}
