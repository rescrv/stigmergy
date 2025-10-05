//! Invariant operations for PostgreSQL database.
//!
//! This module provides functions for managing invariants in the PostgreSQL database
//! with automatic timestamp tracking for created_at and updated_at fields.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{DataStoreError, InvariantID};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents an invariant with its metadata.
#[derive(Debug, Clone)]
pub struct InvariantRecord {
    /// The invariant identifier.
    pub invariant_id: InvariantID,
    /// The assertion or condition that must be met.
    pub asserts: String,
    /// When the invariant was created.
    pub created_at: DateTime<Utc>,
    /// When the invariant was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Creates a new invariant in the database.
///
/// The `created_at` and `updated_at` timestamps are automatically set to the current time.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `invariant_id` - The invariant identifier to create
/// * `asserts` - The assertion or condition that must be met
///
/// # Returns
/// * `Ok(())` - Invariant created successfully
/// * `Err(DataStoreError::AlreadyExists)` - Invariant already exists
/// * `Err(DataStoreError::Internal)` - Database error
///
/// # Examples
/// ```no_run
/// # use stigmergy::{InvariantID, sql};
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let invariant_id = InvariantID::new([1u8; 32]);
/// sql::invariants::create(&pool, &invariant_id, "x > 0").await?;
/// # Ok(())
/// # }
/// ```
pub async fn create(pool: &PgPool, invariant_id: &InvariantID, asserts: &str) -> SqlResult<()> {
    let invariant_bytes = invariant_id.as_bytes();

    let result = sqlx::query!(
        r#"
        INSERT INTO invariants (invariant_id, asserts)
        VALUES ($1, $2)
        "#,
        invariant_bytes.as_slice(),
        asserts
    )
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(e) => {
            eprintln!("Database error creating invariant: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves an invariant from the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `invariant_id` - The invariant to retrieve
///
/// # Returns
/// * `Ok(Some(InvariantRecord))` - Invariant found
/// * `Ok(None)` - Invariant not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(pool: &PgPool, invariant_id: &InvariantID) -> SqlResult<Option<InvariantRecord>> {
    let invariant_bytes = invariant_id.as_bytes();

    let result = sqlx::query!(
        r#"
        SELECT invariant_id, asserts, created_at, updated_at
        FROM invariants
        WHERE invariant_id = $1
        "#,
        invariant_bytes.as_slice()
    )
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let invariant_bytes: [u8; 32] = row
                .invariant_id
                .try_into()
                .map_err(|_| DataStoreError::Internal("invalid invariant_id length".to_string()))?;

            Ok(Some(InvariantRecord {
                invariant_id: InvariantID::new(invariant_bytes),
                asserts: row.asserts,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting invariant: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Updates the assertion for an existing invariant.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `invariant_id` - The invariant to update
/// * `asserts` - The new assertion text
///
/// # Returns
/// * `Ok(true)` - Invariant existed and was updated
/// * `Ok(false)` - Invariant did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn update(pool: &PgPool, invariant_id: &InvariantID, asserts: &str) -> SqlResult<bool> {
    let invariant_bytes = invariant_id.as_bytes();

    let result = sqlx::query!(
        r#"
        UPDATE invariants
        SET asserts = $2, updated_at = CURRENT_TIMESTAMP
        WHERE invariant_id = $1
        "#,
        invariant_bytes.as_slice(),
        asserts
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error updating invariant: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes an invariant from the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `invariant_id` - The invariant to delete
///
/// # Returns
/// * `Ok(true)` - Invariant existed and was deleted
/// * `Ok(false)` - Invariant did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(pool: &PgPool, invariant_id: &InvariantID) -> SqlResult<bool> {
    let invariant_bytes = invariant_id.as_bytes();

    let result = sqlx::query!(
        r#"
        DELETE FROM invariants
        WHERE invariant_id = $1
        "#,
        invariant_bytes.as_slice()
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting invariant: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all invariants in the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
///
/// # Returns
/// * `Ok(Vec<InvariantRecord>)` - List of all invariants
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list(pool: &PgPool) -> SqlResult<Vec<InvariantRecord>> {
    let result = sqlx::query!(
        r#"
        SELECT invariant_id, asserts, created_at, updated_at
        FROM invariants
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let mut invariants = Vec::new();
            for row in rows {
                let invariant_bytes: [u8; 32] = row.invariant_id.try_into().map_err(|_| {
                    DataStoreError::Internal("invalid invariant_id length".to_string())
                })?;
                invariants.push(InvariantRecord {
                    invariant_id: InvariantID::new(invariant_bytes),
                    asserts: row.asserts,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                });
            }
            Ok(invariants)
        }
        Err(e) => {
            eprintln!("Database error listing invariants: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_invariant(test_name: &str) -> InvariantID {
        use std::time::{SystemTime, UNIX_EPOCH};
        let pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&pid.to_le_bytes());
        bytes[4..12].copy_from_slice(&now.to_le_bytes());

        let test_bytes = test_name.as_bytes();
        let copy_len = test_bytes.len().min(20);
        bytes[12..12 + copy_len].copy_from_slice(&test_bytes[..copy_len]);

        InvariantID::new(bytes)
    }

    #[tokio::test]
    async fn create_and_get() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("create_and_get");
        let asserts = "x > 0 && y < 100";

        let db_before = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        create(&pool, &invariant_id, asserts).await.unwrap();

        let db_after = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let record = get(&pool, &invariant_id).await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.invariant_id, invariant_id);
        assert_eq!(record.asserts, asserts);
        assert!(record.created_at >= db_before);
        assert!(record.created_at <= db_after);
        assert!(record.updated_at >= db_before);
        assert!(record.updated_at <= db_after);
        assert_eq!(record.created_at, record.updated_at);
    }

    #[tokio::test]
    async fn create_duplicate_fails() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("create_duplicate_fails");

        create(&pool, &invariant_id, "x > 0").await.unwrap();

        let result = create(&pool, &invariant_id, "y > 0").await;
        assert!(matches!(result, Err(DataStoreError::AlreadyExists)));
    }

    #[tokio::test]
    async fn update_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("update_existing");

        create(&pool, &invariant_id, "x > 0").await.unwrap();

        let record_before = get(&pool, &invariant_id).await.unwrap().unwrap();
        assert_eq!(record_before.asserts, "x > 0");
        assert_eq!(record_before.created_at, record_before.updated_at);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let updated = update(&pool, &invariant_id, "y < 100").await.unwrap();
        assert!(updated);

        let record_after = get(&pool, &invariant_id).await.unwrap().unwrap();
        assert_eq!(record_after.asserts, "y < 100");
        assert_eq!(record_after.created_at, record_before.created_at);
        assert!(record_after.updated_at > record_before.updated_at);
    }

    #[tokio::test]
    async fn update_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("update_nonexistent");

        let updated = update(&pool, &invariant_id, "x > 0").await.unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn delete_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("delete_existing");

        create(&pool, &invariant_id, "x > 0").await.unwrap();

        let deleted = delete(&pool, &invariant_id).await.unwrap();
        assert!(deleted);

        let record = get(&pool, &invariant_id).await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant_id = unique_invariant("delete_nonexistent");

        let deleted = delete(&pool, &invariant_id).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn list_multiple() {
        let pool = super::super::tests::setup_test_db().await;
        let invariant1 = unique_invariant("list_multiple_1");
        let invariant2 = unique_invariant("list_multiple_2");
        let invariant3 = unique_invariant("list_multiple_3");

        create(&pool, &invariant1, "x > 0").await.unwrap();
        create(&pool, &invariant2, "y > 0").await.unwrap();
        create(&pool, &invariant3, "z > 0").await.unwrap();

        let invariants = list(&pool).await.unwrap();
        let ids: Vec<_> = invariants.iter().map(|r| r.invariant_id).collect();
        assert!(ids.contains(&invariant1));
        assert!(ids.contains(&invariant2));
        assert!(ids.contains(&invariant3));
    }
}
