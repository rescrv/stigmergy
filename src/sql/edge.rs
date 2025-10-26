//! SQL operations for edge management.
//!
//! This module provides database operations for edges, following the same patterns
//! as other SQL modules in the codebase.

// TODO(claude): Extract the duplicated byte-to-entity conversion logic in list functions
// into a helper function to follow DRY principle. The conversion appears in list_all,
// list_from, list_to, list_labeled, and list_between.

use crate::edge::Edge;
use crate::entity::Entity;
use crate::errors::DataStoreError;
use sqlx::Postgres;
use sqlx::Transaction;

/// Creates a new edge in the database.
pub async fn create(tx: &mut Transaction<'_, Postgres>, edge: &Edge) -> Result<(), DataStoreError> {
    let result = sqlx::query!(
        r#"
        INSERT INTO edges (src_entity, dst_entity, label_entity)
        VALUES ($1, $2, $3)
        "#,
        edge.src.as_bytes(),
        edge.dst.as_bytes(),
        edge.label.as_bytes()
    )
    .execute(&mut **tx)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(DataStoreError::AlreadyExists)
        }
        Err(e) => Err(DataStoreError::from(e)),
    }
}

/// Deletes a specific edge from the database.
pub async fn delete(
    tx: &mut Transaction<'_, Postgres>,
    src: &Entity,
    dst: &Entity,
    label: &Entity,
) -> Result<(), DataStoreError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM edges
        WHERE src_entity = $1 AND dst_entity = $2 AND label_entity = $3
        "#,
        src.as_bytes(),
        dst.as_bytes(),
        label.as_bytes()
    )
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        Err(DataStoreError::NotFound)
    } else {
        Ok(())
    }
}

/// Gets a specific edge from the database.
pub async fn get(
    tx: &mut Transaction<'_, Postgres>,
    src: &Entity,
    dst: &Entity,
    label: &Entity,
) -> Result<Edge, DataStoreError> {
    let row = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        WHERE src_entity = $1 AND dst_entity = $2 AND label_entity = $3
        "#,
        src.as_bytes(),
        dst.as_bytes(),
        label.as_bytes()
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DataStoreError::NotFound)?;

    edge_from_row_bytes(
        row.src_entity.as_slice(),
        row.dst_entity.as_slice(),
        row.label_entity.as_slice(),
    )
}

/// Lists all edges in the database.
pub async fn list_all(tx: &mut Transaction<'_, Postgres>) -> Result<Vec<Edge>, DataStoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        ORDER BY created_at
        "#
    )
    .fetch_all(&mut **tx)
    .await?;

    rows
        .into_iter()
        .map(|row| {
            edge_from_row_bytes(
                row.src_entity.as_slice(),
                row.dst_entity.as_slice(),
                row.label_entity.as_slice(),
            )
        })
        .collect()
}

/// Lists all edges from a specific source entity.
pub async fn list_from(
    tx: &mut Transaction<'_, Postgres>,
    src: &Entity,
) -> Result<Vec<Edge>, DataStoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        WHERE src_entity = $1
        ORDER BY created_at
        "#,
        src.as_bytes()
    )
    .fetch_all(&mut **tx)
    .await?;

    rows
        .into_iter()
        .map(|row| {
            edge_from_row_bytes(
                row.src_entity.as_slice(),
                row.dst_entity.as_slice(),
                row.label_entity.as_slice(),
            )
        })
        .collect()
}

/// Lists all edges to a specific destination entity.
pub async fn list_to(
    tx: &mut Transaction<'_, Postgres>,
    dst: &Entity,
) -> Result<Vec<Edge>, DataStoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        WHERE dst_entity = $1
        ORDER BY created_at
        "#,
        dst.as_bytes()
    )
    .fetch_all(&mut **tx)
    .await?;

    rows
        .into_iter()
        .map(|row| {
            edge_from_row_bytes(
                row.src_entity.as_slice(),
                row.dst_entity.as_slice(),
                row.label_entity.as_slice(),
            )
        })
        .collect()
}

/// Lists all edges with a specific label entity.
pub async fn list_labeled(
    tx: &mut Transaction<'_, Postgres>,
    label: &Entity,
) -> Result<Vec<Edge>, DataStoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        WHERE label_entity = $1
        ORDER BY created_at
        "#,
        label.as_bytes()
    )
    .fetch_all(&mut **tx)
    .await?;

    rows
        .into_iter()
        .map(|row| {
            edge_from_row_bytes(
                row.src_entity.as_slice(),
                row.dst_entity.as_slice(),
                row.label_entity.as_slice(),
            )
        })
        .collect()
}

/// Lists all edges between two specific entities.
pub async fn list_between(
    tx: &mut Transaction<'_, Postgres>,
    src: &Entity,
    dst: &Entity,
) -> Result<Vec<Edge>, DataStoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT src_entity, dst_entity, label_entity
        FROM edges
        WHERE src_entity = $1 AND dst_entity = $2
        ORDER BY created_at
        "#,
        src.as_bytes(),
        dst.as_bytes()
    )
    .fetch_all(&mut **tx)
    .await?;

    rows
        .into_iter()
        .map(|row| {
            edge_from_row_bytes(
                row.src_entity.as_slice(),
                row.dst_entity.as_slice(),
                row.label_entity.as_slice(),
            )
        })
        .collect()
}

fn edge_from_row_bytes(
    src_bytes: &[u8],
    dst_bytes: &[u8],
    label_bytes: &[u8],
) -> Result<Edge, DataStoreError> {
    Ok(Edge {
        src: entity_from_bytes(src_bytes, "src entity")?,
        dst: entity_from_bytes(dst_bytes, "dst entity")?,
        label: entity_from_bytes(label_bytes, "label entity")?,
    })
}

fn entity_from_bytes(bytes: &[u8], field_name: &'static str) -> Result<Entity, DataStoreError> {
    let entity_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| DataStoreError::Internal(format!("invalid {} bytes", field_name)))?;

    Ok(Entity::new(entity_bytes))
}
