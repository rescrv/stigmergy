//! Component instance operations for PostgreSQL database.
//!
//! This module provides functions for managing component instances in the PostgreSQL database
//! with automatic timestamp tracking for created_at and updated_at fields.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Postgres, Transaction};

use crate::{Component, DataStoreError, Entity};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents a component instance with its metadata.
#[derive(Debug, Clone)]
pub struct ComponentRecord {
    /// The entity this component is attached to.
    pub entity: Entity,
    /// The component type.
    pub component: Component,
    /// The component data.
    pub data: Value,
    /// When the component instance was created.
    pub created_at: DateTime<Utc>,
    /// When the component instance was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Creates a new component instance in the database.
///
/// The `created_at` and `updated_at` timestamps are automatically set to the current time.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to attach the component to
/// * `component` - The component type
/// * `data` - The component data (must be valid against the component definition schema)
///
/// # Returns
/// * `Ok(())` - Component instance created successfully
/// * `Err(DataStoreError::AlreadyExists)` - Component instance already exists for this entity
/// * `Err(DataStoreError::Internal)` - Database error
///
/// # Examples
/// ```no_run
/// # use stigmergy::{Entity, Component, sql};
/// # use serde_json::json;
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let entity = Entity::new([1u8; 32]);
/// let component = Component::new("Position").unwrap();
/// let data = json!({"x": 1.0, "y": 2.0, "z": 3.0});
/// let mut tx = pool.begin().await?;
/// sql::component::create(&mut tx, &entity, &component, &data).await?;
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
pub async fn create(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    component: &Component,
    data: &Value,
) -> SqlResult<()> {
    let entity_bytes = entity.as_bytes();
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        INSERT INTO component_instances (entity_id, component_name, data)
        VALUES ($1, $2, $3)
        "#,
        entity_bytes.as_slice(),
        component_name,
        data
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_foreign_key_violation() => {
            Err(DataStoreError::NotFound)
        }
        Err(e) => {
            eprintln!("Database error creating component instance: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves a component instance from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to retrieve the component from
/// * `component` - The component type
///
/// # Returns
/// * `Ok(Some(Value))` - Component instance found
/// * `Ok(None)` - Component instance not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    component: &Component,
) -> SqlResult<Option<Value>> {
    let entity_bytes = entity.as_bytes();
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        SELECT data
        FROM component_instances
        WHERE entity_id = $1 AND component_name = $2
        "#,
        entity_bytes.as_slice(),
        component_name
    )
    .fetch_optional(&mut **tx)
    .await;

    match result {
        Ok(Some(row)) => Ok(row.data),
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting component instance: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Updates an existing component instance in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity the component is attached to
/// * `component` - The component type
/// * `data` - The new component data
///
/// # Returns
/// * `Ok(true)` - Component instance existed and was updated
/// * `Ok(false)` - Component instance did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn update(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    component: &Component,
    data: &Value,
) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        UPDATE component_instances
        SET data = $3, updated_at = CURRENT_TIMESTAMP
        WHERE entity_id = $1 AND component_name = $2
        "#,
        entity_bytes.as_slice(),
        component_name,
        data
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error updating component instance: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Upserts a component instance in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to attach the component to
/// * `component` - The component type
/// * `data` - The component data
///
/// # Returns
/// * `Ok(true)` - Component instance was created (didn't exist before)
/// * `Ok(false)` - Component instance was updated (existed before)
/// * `Err(DataStoreError::NotFound)` - Entity or component definition not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn upsert(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    component: &Component,
    data: &Value,
) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        INSERT INTO component_instances (entity_id, component_name, data)
        VALUES ($1, $2, $3)
        ON CONFLICT (entity_id, component_name) 
        DO UPDATE SET data = EXCLUDED.data, updated_at = CURRENT_TIMESTAMP
        RETURNING (xmax = 0) as "was_insert!"
        "#,
        entity_bytes.as_slice(),
        component_name,
        data
    )
    .fetch_one(&mut **tx)
    .await;

    match result {
        Ok(row) => Ok(row.was_insert),
        Err(sqlx::Error::Database(db_err)) if db_err.is_foreign_key_violation() => {
            Err(DataStoreError::NotFound)
        }
        Err(e) => {
            eprintln!("Database error upserting component instance: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes a component instance from the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity the component is attached to
/// * `component` - The component type
///
/// # Returns
/// * `Ok(true)` - Component instance existed and was deleted
/// * `Ok(false)` - Component instance did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
    component: &Component,
) -> SqlResult<bool> {
    let entity_bytes = entity.as_bytes();
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        DELETE FROM component_instances
        WHERE entity_id = $1 AND component_name = $2
        "#,
        entity_bytes.as_slice(),
        component_name
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting component instance: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all component instances for a specific entity.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to list components for
///
/// # Returns
/// * `Ok(Vec<(Component, Value)>)` - List of components and their data
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list_for_entity(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
) -> SqlResult<Vec<(Component, Value)>> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        SELECT component_name, data
        FROM component_instances
        WHERE entity_id = $1
        ORDER BY component_name ASC
        "#,
        entity_bytes.as_slice()
    )
    .fetch_all(&mut **tx)
    .await;

    match result {
        Ok(rows) => {
            let mut components = Vec::new();
            for row in rows {
                let component = Component::new(&row.component_name).ok_or_else(|| {
                    DataStoreError::Internal(format!(
                        "invalid component name: {}",
                        row.component_name
                    ))
                })?;
                if let Some(data) = row.data {
                    components.push((component, data));
                }
            }
            Ok(components)
        }
        Err(e) => {
            eprintln!("Database error listing component instances: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all component instances in the database.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
///
/// # Returns
/// * `Ok(Vec<((Entity, Component), Value)>)` - List of all component instances
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list_all(
    tx: &mut Transaction<'_, Postgres>,
) -> SqlResult<Vec<((Entity, Component), Value)>> {
    let result = sqlx::query!(
        r#"
        SELECT entity_id, component_name, data
        FROM component_instances
        ORDER BY entity_id ASC, component_name ASC
        "#
    )
    .fetch_all(&mut **tx)
    .await;

    match result {
        Ok(rows) => {
            let mut components = Vec::new();
            for row in rows {
                let entity_bytes: [u8; 32] = row.entity_id.try_into().map_err(|_| {
                    DataStoreError::Internal("invalid entity_id length".to_string())
                })?;
                let entity = Entity::new(entity_bytes);

                let component = Component::new(&row.component_name).ok_or_else(|| {
                    DataStoreError::Internal(format!(
                        "invalid component name: {}",
                        row.component_name
                    ))
                })?;

                if let Some(data) = row.data {
                    components.push(((entity, component), data));
                }
            }
            Ok(components)
        }
        Err(e) => {
            eprintln!("Database error listing all component instances: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes all component instances for a specific entity.
///
/// # Arguments
/// * `tx` - PostgreSQL transaction
/// * `entity` - The entity to delete all components from
///
/// # Returns
/// * `Ok(count)` - Number of component instances deleted
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete_all_for_entity(
    tx: &mut Transaction<'_, Postgres>,
    entity: &Entity,
) -> SqlResult<u32> {
    let entity_bytes = entity.as_bytes();

    let result = sqlx::query!(
        r#"
        DELETE FROM component_instances
        WHERE entity_id = $1
        "#,
        entity_bytes.as_slice()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() as u32),
        Err(e) => {
            eprintln!("Database error deleting all component instances: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

        let entity = unique_entity("component_create_and_get");
        let component = Component::new("Position").unwrap();
        let data = json!({"x": 1.0, "y": 2.0, "z": 3.0});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        let def = crate::ComponentDefinition::new(
            component.clone(),
            json!({"type": "object", "properties": {"x": {"type": "number"}}}),
        );
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();

        create(&mut tx, &entity, &component, &data).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let retrieved = get(&mut tx, &entity, &component).await.unwrap();
        tx.commit().await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
    }

    #[tokio::test]
    async fn create_duplicate_fails() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("component_create_duplicate");
        let component = Component::new("Health").unwrap();
        let data = json!({"hp": 100});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        let def = crate::ComponentDefinition::new(
            component.clone(),
            json!({"type": "object", "properties": {"hp": {"type": "number"}}}),
        );
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();

        create(&mut tx, &entity, &component, &data).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = create(&mut tx, &entity, &component, &data).await;
        assert!(matches!(result, Err(DataStoreError::AlreadyExists)));
    }

    #[tokio::test]
    async fn update_existing() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("component_update");
        let component = Component::new("Score").unwrap();
        let data1 = json!({"points": 100});
        let data2 = json!({"points": 200});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        let def = crate::ComponentDefinition::new(
            component.clone(),
            json!({"type": "object", "properties": {"points": {"type": "number"}}}),
        );
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();

        create(&mut tx, &entity, &component, &data1).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let updated = update(&mut tx, &entity, &component, &data2).await.unwrap();
        tx.commit().await.unwrap();
        assert!(updated);

        let mut tx = pool.begin().await.unwrap();
        let retrieved = get(&mut tx, &entity, &component).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        assert_eq!(retrieved, data2);
    }

    #[tokio::test]
    async fn delete_existing() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("component_delete");
        let component = Component::new("Tag").unwrap();
        let data = json!({"label": "test"});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        let def = crate::ComponentDefinition::new(
            component.clone(),
            json!({"type": "object", "properties": {"label": {"type": "string"}}}),
        );
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();

        create(&mut tx, &entity, &component, &data).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let deleted = delete(&mut tx, &entity, &component).await.unwrap();
        tx.commit().await.unwrap();
        assert!(deleted);

        let mut tx = pool.begin().await.unwrap();
        let retrieved = get(&mut tx, &entity, &component).await.unwrap();
        tx.commit().await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn list_for_entity_multiple() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("component_list");
        let comp1 = Component::new("Alpha").unwrap();
        let comp2 = Component::new("Beta").unwrap();
        let comp3 = Component::new("Gamma").unwrap();
        let data1 = json!({"value": 1});
        let data2 = json!({"value": 2});
        let data3 = json!({"value": 3});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        for comp in [&comp1, &comp2, &comp3] {
            let def = crate::ComponentDefinition::new(
                comp.clone(),
                json!({"type": "object", "properties": {"value": {"type": "number"}}}),
            );
            crate::sql::component_definition::create(&mut tx, &def)
                .await
                .unwrap();
        }

        create(&mut tx, &entity, &comp1, &data1).await.unwrap();
        create(&mut tx, &entity, &comp2, &data2).await.unwrap();
        create(&mut tx, &entity, &comp3, &data3).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let components = list_for_entity(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(components.len(), 3);

        let component_names: Vec<_> = components.iter().map(|(c, _)| c).collect();
        assert!(component_names.contains(&&comp1));
        assert!(component_names.contains(&&comp2));
        assert!(component_names.contains(&&comp3));
    }

    #[tokio::test]
    async fn delete_all_for_entity_test() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("component_delete_all");
        let comp1 = Component::new("One").unwrap();
        let comp2 = Component::new("Two").unwrap();
        let data = json!({"x": 1});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        for comp in [&comp1, &comp2] {
            let def = crate::ComponentDefinition::new(
                comp.clone(),
                json!({"type": "object", "properties": {"x": {"type": "number"}}}),
            );
            crate::sql::component_definition::create(&mut tx, &def)
                .await
                .unwrap();
        }

        create(&mut tx, &entity, &comp1, &data).await.unwrap();
        create(&mut tx, &entity, &comp2, &data).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let count = delete_all_for_entity(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(count, 2);

        let mut tx = pool.begin().await.unwrap();
        let components = list_for_entity(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(components.is_empty());
    }

    #[tokio::test]
    async fn upsert_returns_correct_created_flag() {
        let pool = super::super::tests::setup_test_db().await;

        let entity = unique_entity("upsert_created_flag");
        let component = Component::new("StatusFlag").unwrap();
        let data1 = json!({"status": "active"});
        let data2 = json!({"status": "inactive"});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();

        let def = crate::ComponentDefinition::new(
            component.clone(),
            json!({"type": "object", "properties": {"status": {"type": "string"}}}),
        );
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();

        let created_first = upsert(&mut tx, &entity, &component, &data1).await.unwrap();
        println!("First upsert created flag: {}", created_first);
        assert!(created_first);

        let created_second = upsert(&mut tx, &entity, &component, &data2).await.unwrap();
        println!("Second upsert created flag: {}", created_second);
        assert!(!created_second);

        let retrieved = get(&mut tx, &entity, &component).await.unwrap().unwrap();
        assert_eq!(retrieved, data2);

        tx.commit().await.unwrap();
    }
}
