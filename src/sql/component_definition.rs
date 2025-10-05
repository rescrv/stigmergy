//! Component definition operations for PostgreSQL database.
//!
//! This module provides functions for managing component definitions in the PostgreSQL database
//! with automatic timestamp tracking for created_at and updated_at fields.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{Component, ComponentDefinition, DataStoreError};

/// Result type for database operations.
pub type SqlResult<T> = Result<T, DataStoreError>;

/// Represents a component definition with its metadata.
#[derive(Debug, Clone)]
pub struct ComponentDefinitionRecord {
    /// The component definition.
    pub definition: ComponentDefinition,
    /// When the component definition was created.
    pub created_at: DateTime<Utc>,
    /// When the component definition was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Creates a new component definition in the database.
///
/// The `created_at` and `updated_at` timestamps are automatically set to the current time.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `definition` - The component definition to create
///
/// # Returns
/// * `Ok(())` - Component definition created successfully
/// * `Err(DataStoreError::AlreadyExists)` - Component definition already exists
/// * `Err(DataStoreError::Internal)` - Database error
///
/// # Examples
/// ```no_run
/// # use stigmergy::{Component, ComponentDefinition, sql};
/// # use serde_json::json;
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let component = Component::new("Position").unwrap();
/// let definition = ComponentDefinition::new(
///     component.clone(),
///     json!({"type": "object"})
/// );
/// sql::component_definition::create(&pool, &definition).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create(pool: &PgPool, definition: &ComponentDefinition) -> SqlResult<()> {
    let component_name = definition.component.as_str();
    let schema = serde_json::to_value(&definition.schema)
        .map_err(|e| DataStoreError::SerializationError(e.to_string()))?;

    let result = sqlx::query!(
        r#"
        INSERT INTO component_definitions (component_name, schema)
        VALUES ($1, $2)
        "#,
        component_name,
        schema
    )
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(e) => {
            eprintln!("Database error creating component definition: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Retrieves a component definition from the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `component` - The component type identifier
///
/// # Returns
/// * `Ok(Some(ComponentDefinitionRecord))` - Component definition found
/// * `Ok(None)` - Component definition not found
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn get(
    pool: &PgPool,
    component: &Component,
) -> SqlResult<Option<ComponentDefinitionRecord>> {
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        SELECT component_name, schema, created_at, updated_at
        FROM component_definitions
        WHERE component_name = $1
        "#,
        component_name
    )
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let component = Component::new(&row.component_name).ok_or_else(|| {
                DataStoreError::Internal(format!("invalid component name: {}", row.component_name))
            })?;

            let definition = ComponentDefinition::new(component, row.schema);

            Ok(Some(ComponentDefinitionRecord {
                definition,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Database error getting component definition: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Updates an existing component definition in the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `definition` - The new component definition
///
/// # Returns
/// * `Ok(true)` - Component definition existed and was updated
/// * `Ok(false)` - Component definition did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn update(pool: &PgPool, definition: &ComponentDefinition) -> SqlResult<bool> {
    let component_name = definition.component.as_str();
    let schema = serde_json::to_value(&definition.schema)
        .map_err(|e| DataStoreError::SerializationError(e.to_string()))?;

    let result = sqlx::query!(
        r#"
        UPDATE component_definitions
        SET schema = $2, updated_at = CURRENT_TIMESTAMP
        WHERE component_name = $1
        "#,
        component_name,
        schema
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error updating component definition: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Deletes a component definition from the database.
///
/// This will cascade delete all associated component instances and messages.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `component` - The component type identifier
///
/// # Returns
/// * `Ok(true)` - Component definition existed and was deleted
/// * `Ok(false)` - Component definition did not exist
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn delete(pool: &PgPool, component: &Component) -> SqlResult<bool> {
    let component_name = component.as_str();

    let result = sqlx::query!(
        r#"
        DELETE FROM component_definitions
        WHERE component_name = $1
        "#,
        component_name
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => Ok(result.rows_affected() > 0),
        Err(e) => {
            eprintln!("Database error deleting component definition: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

/// Lists all component definitions in the database.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
///
/// # Returns
/// * `Ok(Vec<ComponentDefinition>)` - List of all component definitions
/// * `Err(DataStoreError::Internal)` - Database error
pub async fn list(pool: &PgPool) -> SqlResult<Vec<ComponentDefinition>> {
    let result = sqlx::query!(
        r#"
        SELECT component_name, schema
        FROM component_definitions
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let mut definitions = Vec::new();
            for row in rows {
                let component = Component::new(&row.component_name).ok_or_else(|| {
                    DataStoreError::Internal(format!(
                        "invalid component name: {}",
                        row.component_name
                    ))
                })?;
                let definition = ComponentDefinition::new(component, row.schema);
                definitions.push(definition);
            }
            Ok(definitions)
        }
        Err(e) => {
            eprintln!("Database error listing component definitions: {}", e);
            Err(DataStoreError::Internal(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unique_component(test_name: &str, suffix: u64) -> Component {
        Component::new(format!("{}_{}", test_name, suffix)).unwrap()
    }

    #[tokio::test]
    async fn create_and_get() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("create_and_get", std::process::id() as u64);
        let schema = json!({"type": "object", "properties": {"x": {"type": "number"}}});
        let definition = ComponentDefinition::new(component.clone(), schema);

        let db_before = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        create(&pool, &definition).await.unwrap();

        let db_after = sqlx::query_scalar::<_, DateTime<Utc>>("SELECT CURRENT_TIMESTAMP")
            .fetch_one(&pool)
            .await
            .unwrap();

        let record = get(&pool, &component).await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.definition.component, component);
        assert_eq!(record.definition.schema, definition.schema);
        assert!(record.created_at >= db_before);
        assert!(record.created_at <= db_after);
        assert!(record.updated_at >= db_before);
        assert!(record.updated_at <= db_after);
        assert_eq!(record.created_at, record.updated_at);
    }

    #[tokio::test]
    async fn create_duplicate_fails() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("create_duplicate_fails", std::process::id() as u64);
        let schema = json!({"type": "object"});
        let definition = ComponentDefinition::new(component.clone(), schema);

        create(&pool, &definition).await.unwrap();

        let result = create(&pool, &definition).await;
        assert!(matches!(result, Err(DataStoreError::AlreadyExists)));
    }

    #[tokio::test]
    async fn update_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("update_existing", std::process::id() as u64);
        let schema1 = json!({"type": "object"});
        let definition1 = ComponentDefinition::new(component.clone(), schema1);

        create(&pool, &definition1).await.unwrap();

        let record_before = get(&pool, &component).await.unwrap().unwrap();
        assert_eq!(record_before.created_at, record_before.updated_at);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let schema2 = json!({"type": "string"});
        let definition2 = ComponentDefinition::new(component.clone(), schema2.clone());
        let updated = update(&pool, &definition2).await.unwrap();
        assert!(updated);

        let record_after = get(&pool, &component).await.unwrap().unwrap();
        assert_eq!(record_after.definition.schema, schema2);
        assert_eq!(record_after.created_at, record_before.created_at);
        assert!(record_after.updated_at > record_before.updated_at);
    }

    #[tokio::test]
    async fn update_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("update_nonexistent", std::process::id() as u64);
        let schema = json!({"type": "object"});
        let definition = ComponentDefinition::new(component.clone(), schema);

        let updated = update(&pool, &definition).await.unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn delete_existing() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("delete_existing", std::process::id() as u64);
        let schema = json!({"type": "object"});
        let definition = ComponentDefinition::new(component.clone(), schema);

        create(&pool, &definition).await.unwrap();

        let deleted = delete(&pool, &component).await.unwrap();
        assert!(deleted);

        let record = get(&pool, &component).await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent() {
        let pool = super::super::tests::setup_test_db().await;
        let component = unique_component("delete_nonexistent", std::process::id() as u64);

        let deleted = delete(&pool, &component).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn list_multiple() {
        let pool = super::super::tests::setup_test_db().await;
        let base_id = std::process::id() as u64;
        let component1 = unique_component("list_multiple", base_id);
        let component2 = unique_component("list_multiple", base_id + 1);
        let component3 = unique_component("list_multiple", base_id + 2);

        let schema = json!({"type": "object"});
        let definition1 = ComponentDefinition::new(component1.clone(), schema.clone());
        let definition2 = ComponentDefinition::new(component2.clone(), schema.clone());
        let definition3 = ComponentDefinition::new(component3.clone(), schema);

        create(&pool, &definition1).await.unwrap();
        create(&pool, &definition2).await.unwrap();
        create(&pool, &definition3).await.unwrap();

        let definitions = list(&pool).await.unwrap();
        let components: Vec<_> = definitions.iter().map(|d| &d.component).collect();
        assert!(components.contains(&&component1));
        assert!(components.contains(&&component2));
        assert!(components.contains(&&component3));
    }
}
