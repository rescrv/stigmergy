//! Entity operations for PostgreSQL database.
//!
//! This module provides functions for managing entities in the PostgreSQL database
//! with automatic timestamp tracking for created_at and updated_at fields.

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};

use crate::{DataStoreError, Entity};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents an entity with its metadata.
#[derive(Debug, Clone)]
pub struct EntityRecord {
    /// The entity identifier.
    pub entity: Entity,
    /// When the entity was created.
    pub created_at: DateTime<Utc>,
    /// When the entity was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Creates a new entity in the database.
///
/// The `created_at` and `updated_at` timestamps are automatically set to the current time.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to create
///
/// # Returns
/// * `Ok(())` - Entity created successfully
/// * `Err(DataStoreError::AlreadyExists)` - Entity already exists
/// * `Err(DataStoreError::Internal)` - Database error
///
/// # Examples
/// ```no_run
/// # use stigmergy::{Entity, sql};
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let mut tx = pool.begin().await?;
/// let entity = Entity::new([1u8; 32]);
/// sql::entity::create(&mut tx, &entity).await?;
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
pub async fn create(tx: &mut Transaction<'_, Postgres>, entity: &Entity) -> SqlResult<()> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        INSERT INTO entities (entity_id)
        VALUES ($1)
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(_) => {
            super::active_entity::upsert(tx, entity, None).await?;
            Ok(())
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(e) => {
            eprintln!("Database error creating entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Creates a new entity in the database if it doesn't exist.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to create
///
/// # Returns
/// * `Ok(true)` - Entity was created (didn't exist before)
/// * `Ok(false)` - Entity already exists (no-op)
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn create_idempotent(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        INSERT INTO entities (entity_id)
        VALUES ($1)
        ON CONFLICT (entity_id) DO NOTHING
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => {
            let was_created = result.rows_affected() > 0;
            if was_created {
                super::active_entity::upsert(tx, entity, None).await?;
            }
            Ok(was_created)
        }
        Err(e) => {
            eprintln!("Database error creating entity idempotently: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves an entity from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to retrieve
///
/// # Returns
/// * `Ok(Some(EntityRecord))` - Entity found
/// * `Ok(None)` - Entity not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
) -> SqlResult<Option<EntityRecord>> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        SELECT entity_id, created_at, updated_at
        FROM entities
        WHERE entity_id = $1
        "#,
        entity_bytes.as_slice()
    )
    .fetch_optional(&mut **tx)
    .await;

    match result {
        Ok(Some(row)) => {
            let entity_bytes: [u8; 32] = row
                .entity_id
                .try_into()
                .map_err(|_| DataStoreError::Internal("invalid entity_id length".to_string()))?;

            Ok(Some(EntityRecord {
                entity: Entity::new(entity_bytes),
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes an entity from the database.
///
/// This will cascade delete all associated components, component instances, and messages.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to delete
///
/// # Returns
/// * `Ok(true)` - Entity existed and was deleted
/// * `Ok(false)` - Entity did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(tx: &mut Transaction<'_, Postgres>, entity: &Entity) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        DELETE FROM entities
        WHERE entity_id = $1
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all entities in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
///
/// # Returns
/// * `Ok(Vec<Entity>)` - List of all entities
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list(tx: &mut Transaction<'_, Postgres>) -> SqlResult<Vec<Entity>> {
    let result = sqlx::query!(
        r#"
        SELECT entity_id
        FROM entities
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&mut **tx)
    .await;

    match result {
        Ok(rows) => {
            let mut entities = Vec::new();
            for row in rows {
                let entity_bytes: [u8; 32] = row.entity_id.try_into().map_err(|_| {
                    DataStoreError::Internal("invalid entity_id length".to_string())
                })?;
                entities.push(Entity::new(entity_bytes));
            }
            Ok(entities)
        }
        Err(e) => {
            eprintln!("Database error listing entities: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Updates the `updated_at` timestamp for an entity.
///
/// This is useful when you want to mark an entity as modified without changing its data.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to touch
///
/// # Returns
/// * `Ok(true)` - Entity existed and was updated
/// * `Ok(false)` - Entity did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn touch(tx: &mut Transaction<'_, Postgres>, entity: &Entity) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        UPDATE entities
        SET updated_at = CURRENT_TIMESTAMP
        WHERE entity_id = $1
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error touching entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_entity(test_name: &str) -> Entity {
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

        Entity::new(bytes)
    }

    #[tokio::test]
    async fn create_and_get() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("create_and_get");

        let db_before = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let db_after = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record = get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.entity, entity);
        assert!(record.created_at >= db_before);
        assert!(record.created_at <= db_after);
        assert!(record.updated_at >= db_before);
        assert!(record.updated_at <= db_after);
        assert_eq!(record.created_at, record.updated_at);
    }

    #[tokio::test]
    async fn create_duplicate_fails() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("create_duplicate_fails");

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = create(&mut tx, &entity).await;
        assert!(matches!(result, Err(DataStoreError::AlreadyExists)));
    }

    #[tokio::test]
    async fn delete_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("delete_existing");

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let deleted = delete(&mut tx, &entity).await.unwrap();
        assert!(deleted);
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record = get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("delete_nonexistent");

        let mut tx = pool.begin().await.unwrap();
        let deleted = delete(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn list_multiple() {
        let pool = super::super::tests::setup_test_db().await;
        let entity1 = unique_entity("list_multiple_1");
        let entity2 = unique_entity("list_multiple_2");
        let entity3 = unique_entity("list_multiple_3");

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &entity1).await.unwrap();
        create(&mut tx, &entity2).await.unwrap();
        create(&mut tx, &entity3).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let entities = list(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        assert!(entities.contains(&entity1));
        assert!(entities.contains(&entity2));
        assert!(entities.contains(&entity3));
    }

    #[tokio::test]
    async fn touch_updates_timestamp() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("touch_updates_timestamp");

        let mut tx = pool.begin().await.unwrap();
        create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record_before = get(&mut tx, &entity).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        assert_eq!(record_before.created_at, record_before.updated_at);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut tx = pool.begin().await.unwrap();
        let touched = touch(&mut tx, &entity).await.unwrap();
        assert!(touched);
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record_after = get(&mut tx, &entity).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        assert_eq!(record_after.created_at, record_before.created_at);
        assert!(record_after.updated_at > record_before.updated_at);
    }
}
