//! Active entity operations for PostgreSQL database.
//!
//! This module provides functions for managing active entities in the PostgreSQL database.
//! Active entities are entities that have recently been created or modified.

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};

use crate::{DataStoreError, Entity};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents an active entity record with its metadata.
#[derive(Debug, Clone)]
pub struct ActiveEntityRecord {
    /// The entity identifier.
    pub entity: Entity,
    /// The optional system name.
    pub system_name: Option<String>,
    /// When the active entity record was created.
    pub created_at: DateTime<Utc>,
    /// When the active entity record was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Inserts or updates an active entity record.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to mark as active
/// * `system_name` - Optional system name to associate with the active entity
///
/// # Returns
/// * `Ok(())` - Active entity record created or updated successfully
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn upsert(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    system_name: Option<&str>,
) -> SqlResult<()> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        INSERT INTO active_entity (entity_id, system_name)
        VALUES ($1, $2)
        ON CONFLICT (entity_id)
        DO UPDATE SET system_name = EXCLUDED.system_name, updated_at = CURRENT_TIMESTAMP
        "#,
        entity_bytes.as_slice(),
        system_name
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Database error upserting active entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves the active entity record for a specific entity.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to retrieve the active record for
///
/// # Returns
/// * `Ok(Some(ActiveEntityRecord))` - Active entity record found
/// * `Ok(None)` - Entity is not active
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
) -> SqlResult<Option<ActiveEntityRecord>> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        SELECT entity_id, system_name, created_at, updated_at
        FROM active_entity
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

            Ok(Some(ActiveEntityRecord {
                entity: Entity::new(entity_bytes),
                system_name: row.system_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting active entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves all active entities for a specific system.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `system_name` - The system name to filter by
///
/// # Returns
/// * `Ok(Vec<Entity>)` - List of active entities for the system
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list_for_system(
    tx: &mut Transaction<'_, Postgres>,
    system_name: &str,
) -> SqlResult<Vec<Entity>> {
    let result = sqlx::query!(
        r#"
        SELECT entity_id
        FROM active_entity
        WHERE system_name = $1
        ORDER BY created_at ASC
        "#,
        system_name
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
            eprintln!("Database error listing active entities for system: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes an active entity record.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to remove from active entities
///
/// # Returns
/// * `Ok(true)` - Active entity record existed and was deleted
/// * `Ok(false)` - Active entity record did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(tx: &mut Transaction<'_, Postgres>, entity: &Entity) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        DELETE FROM active_entity
        WHERE entity_id = $1
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting active entity: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all active entities in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
///
/// # Returns
/// * `Ok(Vec<Entity>)` - List of all active entities
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list_all(tx: &mut Transaction<'_, Postgres>) -> SqlResult<Vec<Entity>> {
    let result = sqlx::query!(
        r#"
        SELECT entity_id
        FROM active_entity
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
            eprintln!("Database error listing all active entities: {}", e);
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
    async fn upsert_and_get() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("active_entity_upsert");

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();
        upsert(&mut tx, &entity, None).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record = get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.entity, entity);
        assert_eq!(record.system_name, None);
    }

    #[tokio::test]
    async fn upsert_with_system() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("active_entity_with_system");
        let system_name = "test_system";

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        sqlx::query!(
            "INSERT INTO systems (system_name, model) VALUES ($1, $2)",
            system_name,
            "test_model"
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        upsert(&mut tx, &entity, Some(system_name)).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let record = get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.system_name, Some(system_name.to_string()));
    }

    #[tokio::test]
    async fn delete_active_entity() {
        let pool = super::super::tests::setup_test_db().await;
        let entity = unique_entity("delete_active_entity");

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();
        upsert(&mut tx, &entity, None).await.unwrap();
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
    async fn list_for_system_test() {
        let pool = super::super::tests::setup_test_db().await;
        let entity1 = unique_entity("list_for_system_1");
        let entity2 = unique_entity("list_for_system_2");
        let system_name = "test_system_list";

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity1).await.unwrap();
        crate::sql::entity::create(&mut tx, &entity2).await.unwrap();

        sqlx::query!(
            "INSERT INTO systems (system_name, model) VALUES ($1, $2)",
            system_name,
            "test_model"
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        upsert(&mut tx, &entity1, Some(system_name)).await.unwrap();
        upsert(&mut tx, &entity2, Some(system_name)).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let entities = list_for_system(&mut tx, system_name).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&entity1));
        assert!(entities.contains(&entity2));
    }
}
