//! System operations for PostgreSQL database.
//!
//! This module provides functions for managing systems in the PostgreSQL database
//! with automatic timestamp tracking for created_at and updated_at fields.

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};

use crate::{DataStoreError, System, SystemName};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents a system with its metadata.
#[derive(Debug, Clone)]
pub struct SystemRecord {
    /// The system.
    pub system: System,
    /// When the system was created.
    pub created_at: DateTime<Utc>,
    /// When the system was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Creates a new system in the database.
///
/// The `created_at` and `updated_at` timestamps are automatically set to the current time.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `system` - The system to create
///
/// # Returns
/// * `Ok(())` - System created successfully
/// * `Err(DataStoreError::AlreadyExists)` - System already exists
/// * `Err(DataStoreError::Internal)` - Database error
///
/// # Examples
/// ```no_run
/// # use stigmergy::{System, SystemName, SystemConfig, sql};
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let config = SystemConfig {
///     name: SystemName::new("test-system").unwrap(),
///     description: "Test system".to_string(),
///     model: "inherit".to_string(),
///     color: "blue".to_string(),
///     component: Vec::new(),
///     bid: Vec::new(),
///     content: "You are a test system.".to_string(),
/// };
/// let system = System::new(config);
/// let mut tx = pool.begin().await?;
/// sql::system::create(&mut tx, &system).await?;
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
pub async fn create(tx: &mut Transaction<'_, Postgres>, system: &System) -> SqlResult<()> {
    let system_name = system.name().as_str();
    let description = &system.config.description;
    let model = &system.config.model;
    let color = &system.config.color;
    let content = &system.config.content;
    let bids: Vec<String> = system.config.bid.iter().map(|b| b.to_string()).collect();

    let result = sqlx::query!(
        r#"
        INSERT INTO systems (system_name, description, model, color, content, bids)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        system_name,
        description,
        model,
        color,
        content,
        &bids as &[String]
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(e) => {
            eprintln!("Database error creating system: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves a system from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `name` - The system name identifier
///
/// # Returns
/// * `Ok(Some(System))` - System found
/// * `Ok(None)` - System not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(
    tx: &mut Transaction<'_, Postgres>,
    name: &SystemName,
) -> SqlResult<Option<System>> {
    let system_name = name.as_str();

    let result = sqlx::query!(
        r#"
        SELECT system_name, description, model, color, content, bids, created_at, updated_at
        FROM systems
        WHERE system_name = $1
        "#,
        system_name
    )
    .fetch_optional(&mut **tx)
    .await;

    match result {
        Ok(Some(row)) => {
            let name = SystemName::new(&row.system_name).ok_or_else(|| {
                DataStoreError::Internal(format!("invalid system name: {}", row.system_name))
            })?;

            let mut bids = Vec::new();
            for bid_str in &row.bids {
                let bid = crate::BidParser::parse(bid_str)
                    .map_err(|e| DataStoreError::Internal(format!("failed to parse bid: {}", e)))?;
                bids.push(bid);
            }

            let config = crate::SystemConfig {
                name,
                description: row.description.unwrap_or_default(),
                model: row.model,
                color: row.color.unwrap_or_default(),
                component: Vec::new(),
                bid: bids,
                content: row.content.unwrap_or_default(),
            };

            Ok(Some(System {
                config,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting system: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Updates an existing system in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `system` - The system with updated data
///
/// # Returns
/// * `Ok(true)` - System existed and was updated
/// * `Ok(false)` - System did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn update(tx: &mut Transaction<'_, Postgres>, system: &System) -> SqlResult<bool> {
    let system_name = system.name().as_str();
    let description = &system.config.description;
    let model = &system.config.model;
    let color = &system.config.color;
    let content = &system.config.content;
    let bids: Vec<String> = system.config.bid.iter().map(|b| b.to_string()).collect();

    let result = sqlx::query!(
        r#"
        UPDATE systems
        SET description = $2, model = $3, color = $4, content = $5, bids = $6, updated_at = CURRENT_TIMESTAMP
        WHERE system_name = $1
        "#,
        system_name,
        description,
        model,
        color,
        content,
        &bids as &[String]
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error updating system: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes a system from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `name` - The system name identifier
///
/// # Returns
/// * `Ok(true)` - System existed and was deleted
/// * `Ok(false)` - System did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(tx: &mut Transaction<'_, Postgres>, name: &SystemName) -> SqlResult<bool> {
    let system_name = name.as_str();

    let result = sqlx::query!(
        r#"
        DELETE FROM systems
        WHERE system_name = $1
        "#,
        system_name
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting system: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes all systems from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
///
/// # Returns
/// * `Ok(count)` - Number of systems deleted
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete_all(tx: &mut Transaction<'_, Postgres>) -> SqlResult<u32> {
    let result = sqlx::query!(
        r#"
        DELETE FROM systems
        "#
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() as u32),
        Err(e) => {
            eprintln!("Database error deleting all systems: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all systems in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
///
/// # Returns
/// * `Ok(Vec<System>)` - List of all systems
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list(tx: &mut Transaction<'_, Postgres>) -> SqlResult<Vec<System>> {
    let result = sqlx::query!(
        r#"
        SELECT system_name, description, model, color, content, bids, created_at, updated_at
        FROM systems
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&mut **tx)
    .await;

    match result {
        Ok(rows) => {
            let mut systems = Vec::new();
            for row in rows {
                let name = SystemName::new(&row.system_name).ok_or_else(|| {
                    DataStoreError::Internal(format!("invalid system name: {}", row.system_name))
                })?;

                let mut bids = Vec::new();
                for bid_str in &row.bids {
                    let bid = crate::BidParser::parse(bid_str).map_err(|e| {
                        DataStoreError::Internal(format!("failed to parse bid: {}", e))
                    })?;
                    bids.push(bid);
                }

                let config = crate::SystemConfig {
                    name,
                    description: row.description.unwrap_or_default(),
                    model: row.model,
                    color: row.color.unwrap_or_default(),
                    component: Vec::new(),
                    bid: bids,
                    content: row.content.unwrap_or_default(),
                };

                systems.push(System {
                    config,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                });
            }
            Ok(systems)
        }
        Err(e) => {
            eprintln!("Database error listing systems: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemConfig;

    fn unique_system(test_name: &str, suffix: u64) -> System {
        let name = SystemName::new(format!("{}_{}", test_name, suffix)).expect("valid system name");
        let config = SystemConfig {
            name,
            description: format!("Test system for {}", test_name),
            model: "inherit".to_string(),
            color: "blue".to_string(),
            component: Vec::new(),
            bid: Vec::new(),
            content: "You are a test system.".to_string(),
        };
        System::new(config)
    }

    #[tokio::test]
    async fn create_and_get() {
        let pool = super::super::tests::setup_test_db().await;
        let system = unique_system("create_and_get", std::process::id() as u64);
        let system_name = system.name().clone();

        let db_before = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();

        let db_after = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let retrieved = get(&mut tx, &system_name).await.unwrap();
        tx.commit().await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.config.name, system.config.name);
        assert_eq!(retrieved.config.description, system.config.description);
        assert!(retrieved.created_at >= db_before);
        assert!(retrieved.created_at <= db_after);
        assert!(retrieved.updated_at >= db_before);
        assert!(retrieved.updated_at <= db_after);
        assert_eq!(retrieved.created_at, retrieved.updated_at);
    }

    #[tokio::test]
    async fn create_duplicate_fails() {
        let pool = super::super::tests::setup_test_db().await;
        let system = unique_system("create_duplicate_fails", std::process::id() as u64);

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = create(&mut tx, &system).await;
        assert!(matches!(result, Err(DataStoreError::AlreadyExists)));
    }

    #[tokio::test]
    async fn update_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let mut system = unique_system("update_existing", std::process::id() as u64);
        let system_name = system.name().clone();

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record_before = get(&mut tx, &system_name).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        assert_eq!(record_before.created_at, record_before.updated_at);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        system.config.description = "Updated description".to_string();
        let mut tx = pool.begin().await.unwrap();
        let updated = update(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();
        assert!(updated);

        let mut tx = pool.begin().await.unwrap();
        let record_after = get(&mut tx, &system_name).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        assert_eq!(record_after.config.description, "Updated description");
        assert_eq!(record_after.created_at, record_before.created_at);
        assert!(record_after.updated_at > record_before.updated_at);
    }

    #[tokio::test]
    async fn update_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let system = unique_system("update_nonexistent", std::process::id() as u64);

        let mut tx = pool.begin().await.unwrap();
        let updated = update(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn delete_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let system = unique_system("delete_existing", std::process::id() as u64);
        let system_name = system.name().clone();

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let deleted = delete(&mut tx, &system_name).await.unwrap();
        tx.commit().await.unwrap();
        assert!(deleted);

        let mut tx = pool.begin().await.unwrap();
        let record = get(&mut tx, &system_name).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let system = unique_system("delete_nonexistent", std::process::id() as u64);
        let system_name = system.name().clone();

        let mut tx = pool.begin().await.unwrap();
        let deleted = delete(&mut tx, &system_name).await.unwrap();
        tx.commit().await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn delete_all_systems() {
        let pool = super::super::tests::setup_test_db().await;
        let base_id = std::process::id() as u64;
        let system1 = unique_system("delete_all", base_id);
        let system2 = unique_system("delete_all", base_id + 1);
        let system3 = unique_system("delete_all", base_id + 2);

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system1).await.unwrap();
        create(&mut tx, &system2).await.unwrap();
        create(&mut tx, &system3).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let count = delete_all(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        assert!(count >= 3);

        let mut tx = pool.begin().await.unwrap();
        let systems = list(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        assert!(systems.is_empty());
    }

    #[tokio::test]
    async fn list_multiple() {
        let pool = super::super::tests::setup_test_db().await;
        let base_id = std::process::id() as u64;
        let system1 = unique_system("list_multiple", base_id);
        let system2 = unique_system("list_multiple", base_id + 1);
        let system3 = unique_system("list_multiple", base_id + 2);

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &system1).await.unwrap();
        create(&mut tx, &system2).await.unwrap();
        create(&mut tx, &system3).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let systems = list(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        let system_names: Vec<_> = systems.iter().map(|s| s.name()).collect();
        assert!(system_names.contains(&system1.name()));
        assert!(system_names.contains(&system2.name()));
        assert!(system_names.contains(&system3.name()));
    }
}
