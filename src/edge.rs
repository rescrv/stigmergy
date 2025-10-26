//! # Edge Management for Graph Representation
//!
//! This module implements a graph layer over stigmergy entities, representing relationships
//! as directed, labeled edges between entities.
//!
//! ## Graph Semantics
//!
//! The graph is defined as `G := (V, E)` where:
//! - **V** (vertices) = all entities in the stigmergy system
//! - **E** (edges) = triples `(src, dst, label)` where each element is an entity
//!
//! ## Key Properties
//!
//! 1. **Directed edges**: Each edge has a direction from src to dst
//! 2. **Labeled edges**: Each edge is labeled by an entity
//! 3. **Unique constraint**: Only one edge per `(src, dst, label)` combination
//! 4. **Entity-based labels**: Labels are entities, enabling edge metadata through components
//!
//! ## Metadata Strategy
//!
//! Instead of storing edge properties directly, attach components to the label entity:
//! - **Edge weights**: Add a `Weight` component to the label entity
//! - **Edge types**: The label entity's identity serves as the edge type
//! - **Edge metadata**: Any component can be attached to the label entity
//!
//! ## Query Patterns
//!
//! All six permutations are indexed for efficient queries:
//! 1. `(src, dst, label)` - Find specific edge
//! 2. `(src, label, dst)` - Find edges from src with specific label
//! 3. `(dst, src, label)` - Find edges to dst from specific src
//! 4. `(dst, label, src)` - Find edges to dst with specific label
//! 5. `(label, src, dst)` - Find edges with specific label from src
//! 6. `(label, dst, src)` - Find edges with specific label to dst
//!
//! This enables efficient queries for:
//! - All edges from a vertex
//! - All edges to a vertex
//! - All edges with a specific label
//! - All edges between two vertices
//! - All edges from a vertex with a specific label
//! - All edges to a vertex with a specific label
//!
//! ## Example Usage
//!
//! ```rust
//! # use stigmergy::{Entity, Edge};
//! // Create three entities
//! let alice = Entity::random_url_safe()?;
//! let bob = Entity::random_url_safe()?;
//! let friendship = Entity::random_url_safe()?;
//!
//! // Create edge: alice --[friendship]--> bob
//! let edge = Edge { src: alice, dst: bob, label: friendship };
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{entity::Entity, sql};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;

/// Represents a directed, labeled edge in the graph.
///
/// Each edge connects a source entity to a destination entity with a label entity.
/// The label entity can have components attached to store edge metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// The source entity of the edge.
    pub src: Entity,
    /// The destination entity of the edge.
    pub dst: Entity,
    /// The label entity of the edge.
    pub label: Entity,
}

/// Request to create a new edge.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    /// The source entity of the edge.
    pub src: Entity,
    /// The destination entity of the edge.
    pub dst: Entity,
    /// The label entity of the edge.
    pub label: Entity,
}

/// Response from creating an edge.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEdgeResponse {
    /// The created edge.
    pub edge: Edge,
    /// Whether the edge was newly created (true) or already existed (false).
    pub created: bool,
}

async fn create_edge(
    State(pool): State<PgPool>,
    Json(request): Json<CreateEdgeRequest>,
) -> Result<Json<CreateEdgeResponse>, (StatusCode, &'static str)> {
    let edge = Edge {
        src: request.src,
        dst: request.dst,
        label: request.label,
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match sql::edge::get(&mut tx, &edge.src, &edge.dst, &edge.label).await {
        Ok(existing_edge) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            let response = CreateEdgeResponse {
                edge: existing_edge,
                created: false,
            };
            Ok(Json(response))
        }
        Err(crate::errors::DataStoreError::NotFound) => match sql::edge::create(&mut tx, &edge).await
        {
            Ok(()) => {
                tx.commit().await.map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to commit transaction",
                    )
                })?;
                let response = CreateEdgeResponse {
                    edge,
                    created: true,
                };
                Ok(Json(response))
            }
            Err(_e) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to create edge")),
        },
        Err(_e) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to retrieve edge state")),
    }
}

async fn list_edges(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Edge>>, (StatusCode, &'static str)> {
    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edges = sql::edge::list_all(&mut tx)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "failed to list edges"))?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edges))
}

async fn list_edges_from(
    State(pool): State<PgPool>,
    Path(src): Path<String>,
) -> Result<Json<Vec<Edge>>, (StatusCode, &'static str)> {
    let src_entity =
        Entity::from_str(&src).map_err(|_| (StatusCode::BAD_REQUEST, "invalid source entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edges = sql::edge::list_from(&mut tx, &src_entity)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "failed to list edges"))?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edges))
}

async fn list_edges_to(
    State(pool): State<PgPool>,
    Path(dst): Path<String>,
) -> Result<Json<Vec<Edge>>, (StatusCode, &'static str)> {
    let dst_entity = Entity::from_str(&dst)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid destination entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edges = sql::edge::list_to(&mut tx, &dst_entity)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "failed to list edges"))?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edges))
}

async fn list_edges_labeled(
    State(pool): State<PgPool>,
    Path(label): Path<String>,
) -> Result<Json<Vec<Edge>>, (StatusCode, &'static str)> {
    let label_entity =
        Entity::from_str(&label).map_err(|_| (StatusCode::BAD_REQUEST, "invalid label entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edges = sql::edge::list_labeled(&mut tx, &label_entity)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "failed to list edges"))?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edges))
}

async fn list_edges_between(
    State(pool): State<PgPool>,
    Path((src, dst)): Path<(String, String)>,
) -> Result<Json<Vec<Edge>>, (StatusCode, &'static str)> {
    let src_entity =
        Entity::from_str(&src).map_err(|_| (StatusCode::BAD_REQUEST, "invalid source entity"))?;
    let dst_entity = Entity::from_str(&dst)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid destination entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edges = sql::edge::list_between(&mut tx, &src_entity, &dst_entity)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "failed to list edges"))?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edges))
}

async fn get_edge(
    State(pool): State<PgPool>,
    Path((src, dst, label)): Path<(String, String, String)>,
) -> Result<Json<Edge>, (StatusCode, &'static str)> {
    let src_entity =
        Entity::from_str(&src).map_err(|_| (StatusCode::BAD_REQUEST, "invalid source entity"))?;
    let dst_entity = Entity::from_str(&dst)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid destination entity"))?;
    let label_entity =
        Entity::from_str(&label).map_err(|_| (StatusCode::BAD_REQUEST, "invalid label entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    let edge = sql::edge::get(&mut tx, &src_entity, &dst_entity, &label_entity)
        .await
        .map_err(|e| match e {
            crate::errors::DataStoreError::NotFound => (StatusCode::NOT_FOUND, "edge not found"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "failed to get edge"),
        })?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(Json(edge))
}

async fn delete_edge(
    State(pool): State<PgPool>,
    Path((src, dst, label)): Path<(String, String, String)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let src_entity =
        Entity::from_str(&src).map_err(|_| (StatusCode::BAD_REQUEST, "invalid source entity"))?;
    let dst_entity = Entity::from_str(&dst)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid destination entity"))?;
    let label_entity =
        Entity::from_str(&label).map_err(|_| (StatusCode::BAD_REQUEST, "invalid label entity"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    sql::edge::delete(&mut tx, &src_entity, &dst_entity, &label_entity)
        .await
        .map_err(|e| match e {
            crate::errors::DataStoreError::NotFound => (StatusCode::NOT_FOUND, "edge not found"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "failed to delete edge"),
        })?;

    tx.commit().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to commit transaction",
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Creates the HTTP router for edge management endpoints.
pub fn create_edge_router(pool: PgPool) -> Router {
    Router::new()
        .route("/edge", post(create_edge).get(list_edges))
        .route("/edge/from/:src", get(list_edges_from))
        .route("/edge/to/:dst", get(list_edges_to))
        .route("/edge/labeled/:label", get(list_edges_labeled))
        .route("/edge/from/:src/to/:dst", get(list_edges_between))
        .route(
            "/edge/from/:src/to/:dst/labeled/:label",
            get(get_edge).delete(delete_edge),
        )
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql;

    fn unique_entity(prefix: &str) -> Entity {
        let pid = std::process::id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique_str = format!("{}_{}_{}", prefix, pid, timestamp);
        let mut bytes = [0u8; 32];
        bytes[..unique_str.len().min(32)]
            .copy_from_slice(&unique_str.as_bytes()[..unique_str.len().min(32)]);
        Entity::new(bytes)
    }

    #[tokio::test]
    async fn create_edge_success() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("create_edge_src");
        let dst = unique_entity("create_edge_dst");
        let label = unique_entity("create_edge_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let edge = Edge { src, dst, label };

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::create(&mut tx, &edge).await;
        assert!(result.is_ok());
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let retrieved = sql::edge::get(&mut tx, &src, &dst, &label).await.unwrap();
        assert_eq!(retrieved.src, src);
        assert_eq!(retrieved.dst, dst);
        assert_eq!(retrieved.label, label);
    }

    #[tokio::test]
    async fn create_edge_duplicate() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("duplicate_edge_src");
        let dst = unique_entity("duplicate_edge_dst");
        let label = unique_entity("duplicate_edge_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let edge = Edge { src, dst, label };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::create(&mut tx, &edge).await;
        assert!(matches!(
            result,
            Err(crate::errors::DataStoreError::AlreadyExists)
        ));
    }

    #[tokio::test]
    async fn get_edge_not_found() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("get_edge_src");
        let dst = unique_entity("get_edge_dst");
        let label = unique_entity("get_edge_label");

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::get(&mut tx, &src, &dst, &label).await;
        assert!(matches!(
            result,
            Err(crate::errors::DataStoreError::NotFound)
        ));
    }

    #[tokio::test]
    async fn delete_edge_success() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("delete_edge_src");
        let dst = unique_entity("delete_edge_dst");
        let label = unique_entity("delete_edge_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let edge = Edge { src, dst, label };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::delete(&mut tx, &src, &dst, &label).await;
        assert!(result.is_ok());
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::get(&mut tx, &src, &dst, &label).await;
        assert!(matches!(
            result,
            Err(crate::errors::DataStoreError::NotFound)
        ));
    }

    #[tokio::test]
    async fn delete_edge_not_found() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("delete_nf_src");
        let dst = unique_entity("delete_nf_dst");
        let label = unique_entity("delete_nf_label");

        let mut tx = pool.begin().await.unwrap();
        let result = sql::edge::delete(&mut tx, &src, &dst, &label).await;
        assert!(matches!(
            result,
            Err(crate::errors::DataStoreError::NotFound)
        ));
    }

    #[tokio::test]
    async fn list_all_edges() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src1 = unique_entity("list_all_src1");
        let dst1 = unique_entity("list_all_dst1");
        let label1 = unique_entity("list_all_label1");

        let src2 = unique_entity("list_all_src2");
        let dst2 = unique_entity("list_all_dst2");
        let label2 = unique_entity("list_all_label2");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src1).await.unwrap();
        sql::entity::create(&mut tx, &dst1).await.unwrap();
        sql::entity::create(&mut tx, &label1).await.unwrap();
        sql::entity::create(&mut tx, &src2).await.unwrap();
        sql::entity::create(&mut tx, &dst2).await.unwrap();
        sql::entity::create(&mut tx, &label2).await.unwrap();
        tx.commit().await.unwrap();

        let edge1 = Edge {
            src: src1,
            dst: dst1,
            label: label1,
        };
        let edge2 = Edge {
            src: src2,
            dst: dst2,
            label: label2,
        };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge1).await.unwrap();
        sql::edge::create(&mut tx, &edge2).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_all(&mut tx).await.unwrap();
        assert!(edges.len() >= 2);
        assert!(
            edges
                .iter()
                .any(|e| e.src == src1 && e.dst == dst1 && e.label == label1)
        );
        assert!(
            edges
                .iter()
                .any(|e| e.src == src2 && e.dst == dst2 && e.label == label2)
        );
    }

    #[tokio::test]
    async fn list_edges_from() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("list_from_src");
        let dst1 = unique_entity("list_from_dst1");
        let dst2 = unique_entity("list_from_dst2");
        let label1 = unique_entity("list_from_label1");
        let label2 = unique_entity("list_from_label2");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst1).await.unwrap();
        sql::entity::create(&mut tx, &dst2).await.unwrap();
        sql::entity::create(&mut tx, &label1).await.unwrap();
        sql::entity::create(&mut tx, &label2).await.unwrap();
        tx.commit().await.unwrap();

        let edge1 = Edge {
            src,
            dst: dst1,
            label: label1,
        };
        let edge2 = Edge {
            src,
            dst: dst2,
            label: label2,
        };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge1).await.unwrap();
        sql::edge::create(&mut tx, &edge2).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_from(&mut tx, &src).await.unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.src == src));
    }

    #[tokio::test]
    async fn list_edges_to() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src1 = unique_entity("list_to_src1");
        let src2 = unique_entity("list_to_src2");
        let dst = unique_entity("list_to_dst");
        let label1 = unique_entity("list_to_label1");
        let label2 = unique_entity("list_to_label2");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src1).await.unwrap();
        sql::entity::create(&mut tx, &src2).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label1).await.unwrap();
        sql::entity::create(&mut tx, &label2).await.unwrap();
        tx.commit().await.unwrap();

        let edge1 = Edge {
            src: src1,
            dst,
            label: label1,
        };
        let edge2 = Edge {
            src: src2,
            dst,
            label: label2,
        };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge1).await.unwrap();
        sql::edge::create(&mut tx, &edge2).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_to(&mut tx, &dst).await.unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.dst == dst));
    }

    #[tokio::test]
    async fn list_edges_labeled() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src1 = unique_entity("list_labeled_src1");
        let src2 = unique_entity("list_labeled_src2");
        let dst1 = unique_entity("list_labeled_dst1");
        let dst2 = unique_entity("list_labeled_dst2");
        let label = unique_entity("list_labeled_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src1).await.unwrap();
        sql::entity::create(&mut tx, &src2).await.unwrap();
        sql::entity::create(&mut tx, &dst1).await.unwrap();
        sql::entity::create(&mut tx, &dst2).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let edge1 = Edge {
            src: src1,
            dst: dst1,
            label,
        };
        let edge2 = Edge {
            src: src2,
            dst: dst2,
            label,
        };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge1).await.unwrap();
        sql::edge::create(&mut tx, &edge2).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_labeled(&mut tx, &label).await.unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.label == label));
    }

    #[tokio::test]
    async fn list_edges_between() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("list_between_src");
        let dst = unique_entity("list_between_dst");
        let label1 = unique_entity("list_between_label1");
        let label2 = unique_entity("list_between_label2");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label1).await.unwrap();
        sql::entity::create(&mut tx, &label2).await.unwrap();
        tx.commit().await.unwrap();

        let edge1 = Edge {
            src,
            dst,
            label: label1,
        };
        let edge2 = Edge {
            src,
            dst,
            label: label2,
        };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge1).await.unwrap();
        sql::edge::create(&mut tx, &edge2).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_between(&mut tx, &src, &dst).await.unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.src == src && e.dst == dst));
    }

    #[tokio::test]
    async fn edge_cascade_delete() {
        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("cascade_src");
        let dst = unique_entity("cascade_dst");
        let label = unique_entity("cascade_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let edge = Edge { src, dst, label };

        let mut tx = pool.begin().await.unwrap();
        sql::edge::create(&mut tx, &edge).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        sql::entity::delete(&mut tx, &src).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let edges = sql::edge::list_all(&mut tx).await.unwrap();
        assert!(!edges.iter().any(|e| e.src == src));
    }

    #[tokio::test]
    async fn create_edge_idempotent_handler() {
        use axum_test::TestServer;

        let pool = crate::sql::tests::setup_test_db().await;

        let src = unique_entity("handler_idempotent_src");
        let dst = unique_entity("handler_idempotent_dst");
        let label = unique_entity("handler_idempotent_label");

        let mut tx = pool.begin().await.unwrap();
        sql::entity::create(&mut tx, &src).await.unwrap();
        sql::entity::create(&mut tx, &dst).await.unwrap();
        sql::entity::create(&mut tx, &label).await.unwrap();
        tx.commit().await.unwrap();

        let router = create_edge_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let request_body = CreateEdgeRequest { src, dst, label };

        let response = server.post("/edge").json(&request_body).await;

        response.assert_status_ok();
        let first_response: CreateEdgeResponse = response.json();
        println!(
            "create_edge_idempotent_handler first response: {:?}",
            first_response
        );
        assert!(first_response.created);
        assert_eq!(first_response.edge.src, src);
        assert_eq!(first_response.edge.dst, dst);
        assert_eq!(first_response.edge.label, label);

        let response = server.post("/edge").json(&request_body).await;

        response.assert_status_ok();
        let second_response: CreateEdgeResponse = response.json();
        println!(
            "create_edge_idempotent_handler second response: {:?}",
            second_response
        );
        assert!(!second_response.created);
        assert_eq!(second_response.edge.src, src);
        assert_eq!(second_response.edge.dst, dst);
        assert_eq!(second_response.edge.label, label);
    }
}
