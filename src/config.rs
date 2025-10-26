//! Configuration management for stigmergy.io systems.
//!
//! This module provides structures for managing I/O system configurations,
//! including bid expressions, endpoints, and HTTP headers.

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Bid;

/// I/O system configuration with bid expressions, endpoint, and headers.
///
/// Represents a single I/O system that can be configured with multiple
/// bid expressions that determine when and how to interact with the endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IoSystem {
    /// List of bid expressions that determine system behavior.
    pub bid: Vec<Bid>,
    /// HTTPS endpoint URL for the I/O system.
    pub endpoint: String,
    /// HTTP headers to include with requests to the endpoint.
    #[serde(with = "header_map_serde")]
    pub headers: HeaderMap,
}

impl IoSystem {
    /// Creates a new IoSystem with the given configuration.
    ///
    /// # Arguments
    /// * `bid` - List of bid expressions
    /// * `endpoint` - HTTPS endpoint URL
    /// * `headers` - HTTP headers for requests
    pub fn new(bid: Vec<Bid>, endpoint: String, headers: HeaderMap) -> Self {
        Self {
            bid,
            endpoint,
            headers,
        }
    }
}

/// Global configuration containing all I/O system configurations.
///
/// This structure represents the top-level configuration for the stigmergy
/// system, containing all configured I/O systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// List of configured I/O systems.
    pub io_systems: Vec<IoSystem>,
}

impl Config {
    /// Creates a new Config with the given I/O systems.
    ///
    /// # Arguments
    /// * `io_systems` - List of I/O system configurations
    pub fn new(io_systems: Vec<IoSystem>) -> Self {
        Self { io_systems }
    }

    /// Creates an empty Config with no I/O systems.
    pub fn empty() -> Self {
        Self {
            io_systems: Vec::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::empty()
    }
}

/// Response structure for the config GET endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetConfigResponse {
    /// The current configuration.
    pub config: Config,
}

/// Request structure for the config POST endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct PostConfigRequest {
    /// The new configuration to set.
    pub config: Config,
}

/// Response structure for the config POST endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct PostConfigResponse {
    /// Whether the configuration was successfully updated.
    pub updated: bool,
    /// The updated configuration.
    pub config: Config,
    /// The version number of the saved configuration.
    pub version: i64,
}

/// Loads the latest configuration from the database.
///
/// Queries the database for the configuration with the maximum version number.
/// If no configurations exist, returns an empty configuration.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
///
/// # Returns
/// * `Ok(Config)` - The latest configuration
/// * `Err(String)` - If a database error occurs
pub async fn load_latest_config(pool: &PgPool) -> Result<Config, String> {
    let result: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT config_json
        FROM configs
        ORDER BY version DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to load config from database: {}", e))?;

    match result {
        Some((config_json,)) => serde_json::from_value(config_json)
            .map_err(|e| format!("Failed to deserialize config: {}", e)),
        None => Ok(Config::empty()),
    }
}

/// Saves a new configuration version to the database.
///
/// Inserts the configuration into the database with an auto-incrementing version number.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
/// * `config` - Configuration to save
///
/// # Returns
/// * `Ok(i64)` - The version number of the saved configuration
/// * `Err(String)` - If a database error occurs
pub async fn save_config(pool: &PgPool, config: &Config) -> Result<i64, String> {
    let config_json =
        serde_json::to_value(config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    let result: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO configs (config_json)
        VALUES ($1)
        RETURNING version
        "#,
    )
    .bind(config_json)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to save config to database: {}", e))?;

    Ok(result.0)
}

/// HTTP endpoint for retrieving the current configuration.
///
/// This endpoint returns the current I/O system configuration from the database.
///
/// # Returns
/// * `Ok(Json<GetConfigResponse>)` - JSON response with current config on success
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If an error occurs
///
/// # Response Format
/// ```json
/// {
///   "config": {
///     "io_systems": [
///       {
///         "bid": [...],
///         "endpoint": "https://example.com/api",
///         "headers": {...}
///       }
///     ]
///   }
/// }
/// ```
async fn get_config(
    State(pool): State<PgPool>,
) -> Result<Json<GetConfigResponse>, (StatusCode, &'static str)> {
    let config = load_latest_config(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to load config"))?;
    let response = GetConfigResponse { config };
    Ok(Json(response))
}

/// HTTP endpoint for updating the configuration.
///
/// This endpoint accepts a POST request with a new configuration and
/// saves it to the database with an incremented version number.
///
/// # Request Format
/// ```json
/// {
///   "config": {
///     "io_systems": [
///       {
///         "bid": [...],
///         "endpoint": "https://example.com/api",
///         "headers": {...}
///       }
///     ]
///   }
/// }
/// ```
///
/// # Response Format
/// ```json
/// {
///   "updated": true,
///   "config": {
///     "io_systems": [...]
///   },
///   "version": 42
/// }
/// ```
///
/// # Returns
/// * `Ok(Json<PostConfigResponse>)` - JSON response with updated config on success
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If an error occurs
async fn post_config(
    State(pool): State<PgPool>,
    Json(request): Json<PostConfigRequest>,
) -> Result<Json<PostConfigResponse>, (StatusCode, &'static str)> {
    let version = save_config(&pool, &request.config)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to save config"))?;
    let response = PostConfigResponse {
        updated: true,
        config: request.config,
        version,
    };
    Ok(Json(response))
}

/// Creates an Axum router with config endpoints.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool for database operations
///
/// # Routes
/// - `GET /config` - Get the current configuration from the database
/// - `POST /config` - Update the configuration in the database
///
/// # Returns
/// An Axum `Router` configured with the config endpoints and state.
pub fn create_config_router(pool: PgPool) -> Router {
    Router::new()
        .route("/config", get(get_config).post(post_config))
        .with_state(pool)
}

/// Custom serialization for HeaderMap to JSON.
mod header_map_serde {
    use axum::http::{HeaderMap, HeaderName, HeaderValue};
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeMap};
    use serde_json::{Map, Value};

    pub fn serialize<S>(headers: &HeaderMap, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(headers.len()))?;
        for (name, value) in headers.iter() {
            let key = name.as_str();
            let val = value.to_str().unwrap_or("");
            map.serialize_entry(key, val)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HeaderMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = Map::deserialize(deserializer)?;
        let mut headers = HeaderMap::new();
        for (key, value) in map {
            if let Value::String(val) = value
                && let Ok(name) = key.parse::<HeaderName>()
                && let Ok(val) = val.parse::<HeaderValue>()
            {
                headers.insert(name, val);
            }
        }
        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BidParser;

    #[test]
    fn empty_config() {
        let config = Config::empty();
        assert_eq!(config.io_systems.len(), 0);
    }

    #[test]
    fn config_with_io_systems() {
        let bid1 = BidParser::parse("ON true BID 100").unwrap();
        let bid2 = BidParser::parse("ON user.active BID user.score * 10").unwrap();

        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer token123".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());

        let io_system = IoSystem::new(
            vec![bid1, bid2],
            "https://example.com/api".to_string(),
            headers.clone(),
        );

        let config = Config::new(vec![io_system]);
        assert_eq!(config.io_systems.len(), 1);
        assert_eq!(config.io_systems[0].bid.len(), 2);
        assert_eq!(config.io_systems[0].endpoint, "https://example.com/api");
        assert_eq!(config.io_systems[0].headers.len(), 2);
    }

    #[test]
    fn io_system_serialization() {
        let bid = BidParser::parse("ON true BID 42").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "secret".parse().unwrap());

        let io_system = IoSystem::new(vec![bid], "https://api.example.com".to_string(), headers);

        let json = serde_json::to_string(&io_system).unwrap();
        let deserialized: IoSystem = serde_json::from_str(&json).unwrap();

        assert_eq!(io_system.bid.len(), deserialized.bid.len());
        assert_eq!(io_system.endpoint, deserialized.endpoint);
        assert_eq!(
            io_system.headers.get("x-api-key").unwrap(),
            deserialized.headers.get("x-api-key").unwrap()
        );
    }

    #[test]
    fn config_serialization_round_trip() {
        let bid = BidParser::parse("ON price > 100 BID price * 0.9").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer xyz".parse().unwrap());

        let io_system = IoSystem::new(
            vec![bid],
            "https://service.example.com/v1".to_string(),
            headers,
        );

        let config = Config::new(vec![io_system]);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(config.io_systems.len(), deserialized.io_systems.len());
        assert_eq!(
            config.io_systems[0].endpoint,
            deserialized.io_systems[0].endpoint
        );
    }
}
