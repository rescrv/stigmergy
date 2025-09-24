//! # HTTP Client Utilities
//!
//! This module provides a standardized HTTP client for interacting with stigmergy
//! REST API services. It handles common HTTP operations, error handling, and
//! response processing with automatic JSON serialization/deserialization.
//!
//! ## Key Features
//!
//! - **RESTful Operations**: Support for GET, POST, PUT, DELETE HTTP methods
//! - **JSON Handling**: Automatic serialization of request bodies and deserialization of responses
//! - **Error Handling**: Comprehensive error handling with meaningful error messages
//! - **URL Construction**: Automatic API URL construction with consistent versioning
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::http_utils::StigmergyClient;
//!
//! # async {
//! let client = StigmergyClient::new("http://localhost:3000".to_string());
//!
//! // Make a GET request
//! let entities: Vec<String> = client.get("entity").await.unwrap();
//!
//! // Make a POST request with JSON body
//! let request_data = serde_json::json!({"name": "test"});
//! let response: serde_json::Value = client.post("component", &request_data).await.unwrap();
//! # };
//! ```

use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;

use crate::cli_utils;

/// HTTP error type for stigmergy client operations.
///
/// This error type wraps HTTP-related errors that can occur during API
/// communication, providing a consistent error interface.
#[derive(Debug)]
pub struct HttpError {
    /// The error message describing what went wrong
    message: String,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for HttpError {}

/// HTTP client for communicating with stigmergy API services.
///
/// This client provides a high-level interface for making HTTP requests to
/// stigmergy REST APIs. It handles URL construction, JSON serialization,
/// error handling, and response processing automatically.
///
/// # Examples
///
/// ```rust
/// use stigmergy::http_utils::StigmergyClient;
///
/// let client = StigmergyClient::new("http://localhost:3000".to_string());
/// // Client is ready to make API requests
/// ```
pub struct StigmergyClient {
    /// The underlying HTTP client for making requests
    client: Client,
    /// The base URL for the stigmergy API service
    base_url: String,
}

impl StigmergyClient {
    /// Creates a new StigmergyClient with the specified base URL.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the stigmergy API service (e.g., "http://localhost:3000")
    ///
    /// # Examples
    /// ```
    /// use stigmergy::http_utils::StigmergyClient;
    ///
    /// let client = StigmergyClient::new("https://api.example.com".to_string());
    /// ```
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Constructs a full API URL from a relative path.
    ///
    /// This method automatically adds the API version prefix (/api/v1) to create
    /// properly versioned API URLs.
    ///
    /// # Arguments
    /// * `path` - The API path (e.g., "entity", "component/Position")
    ///
    /// # Returns
    /// The complete API URL including base URL and version prefix
    ///
    /// # Examples
    /// ```
    /// use stigmergy::http_utils::StigmergyClient;
    ///
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// let url = client.api_url("entity");
    /// assert_eq!(url, "http://localhost:3000/api/v1/entity");
    /// ```
    pub fn api_url(&self, path: &str) -> String {
        let path = path.strip_prefix('/').unwrap_or(path);
        format!("{}/api/v1/{}", self.base_url, path)
    }

    /// Performs a GET request to the specified API path.
    ///
    /// # Type Parameters
    /// * `T` - The expected response type, must implement `DeserializeOwned`
    ///
    /// # Arguments
    /// * `path` - The API path to request
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized response data
    /// * `Err(Box<dyn Error>)` - Network, HTTP, or deserialization error
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::http_utils::StigmergyClient;
    /// # async {
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// let entities: Vec<String> = client.get("entity").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # };
    /// ```
    pub async fn get<T>(&self, path: &str) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Performs a POST request with JSON body to the specified API path.
    ///
    /// # Type Parameters
    /// * `B` - The request body type, must implement `Serialize`
    /// * `T` - The expected response type, must implement `DeserializeOwned`
    ///
    /// # Arguments
    /// * `path` - The API path to request
    /// * `body` - The request body to serialize as JSON
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized response data
    /// * `Err(Box<dyn Error>)` - Network, HTTP, or serialization/deserialization error
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::http_utils::StigmergyClient;
    /// # use serde_json::json;
    /// # async {
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// let request = json!({"entity": null});
    /// let response: serde_json::Value = client.post("entity", &request).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # };
    /// ```
    pub async fn post<B, T>(&self, path: &str, body: &B) -> Result<T, Box<dyn Error>>
    where
        B: serde::Serialize,
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.post(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    /// Performs a POST request without a body to the specified API path.
    ///
    /// This method is useful for POST endpoints that don't require request data,
    /// such as triggering actions or creating resources with default values.
    ///
    /// # Type Parameters
    /// * `T` - The expected response type, must implement `DeserializeOwned`
    ///
    /// # Arguments
    /// * `path` - The API path to request
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized response data
    /// * `Err(Box<dyn Error>)` - Network, HTTP, or deserialization error
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::http_utils::StigmergyClient;
    /// # async {
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// let response: serde_json::Value = client.post_empty("trigger").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # };
    /// ```
    pub async fn post_empty<T>(&self, path: &str) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Performs a PUT request with JSON body to the specified API path.
    ///
    /// # Type Parameters
    /// * `B` - The request body type, must implement `Serialize`
    /// * `T` - The expected response type, must implement `DeserializeOwned`
    ///
    /// # Arguments
    /// * `path` - The API path to request
    /// * `body` - The request body to serialize as JSON
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized response data
    /// * `Err(Box<dyn Error>)` - Network, HTTP, or serialization/deserialization error
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::http_utils::StigmergyClient;
    /// # use serde_json::json;
    /// # async {
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// let update_data = json!({"name": "updated"});
    /// let response: serde_json::Value = client.put("component/Position", &update_data).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # };
    /// ```
    pub async fn put<B, T>(&self, path: &str, body: &B) -> Result<T, Box<dyn Error>>
    where
        B: serde::Serialize,
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.put(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    /// Performs a DELETE request to the specified API path.
    ///
    /// This method handles DELETE requests that typically don't return response bodies,
    /// returning success/failure based on HTTP status codes.
    ///
    /// # Arguments
    /// * `path` - The API path to request
    ///
    /// # Returns
    /// * `Ok(())` - The request was successful
    /// * `Err(Box<dyn Error>)` - Network or HTTP error
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::http_utils::StigmergyClient;
    /// # async {
    /// let client = StigmergyClient::new("http://localhost:3000".to_string());
    /// client.delete("entity/ABC123").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # };
    /// ```
    pub async fn delete(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let url = self.api_url(path);
        let response = self.client.delete(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error = response.text().await.unwrap_or_default();
            let msg = if error.is_empty() {
                "No error details".to_string()
            } else {
                error
            };
            Err(Box::new(HttpError { message: msg }))
        }
    }

    /// Handles HTTP response processing with automatic JSON deserialization.
    ///
    /// This internal method processes HTTP responses, deserializing successful
    /// responses to the expected type or converting error responses to HttpError.
    ///
    /// # Type Parameters
    /// * `T` - The expected response type, must implement `DeserializeOwned`
    ///
    /// # Arguments
    /// * `response` - The HTTP response to process
    ///
    /// # Returns
    /// * `Ok(T)` - Successfully deserialized response data
    /// * `Err(Box<dyn Error>)` - HTTP error or deserialization failure
    async fn handle_response<T>(&self, response: Response) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error = response.text().await.unwrap_or_default();
            let msg = if error.is_empty() {
                "No error details".to_string()
            } else {
                error
            };
            Err(Box::new(HttpError { message: msg }))
        }
    }
}

/// Executes an HTTP operation and terminates the program on error.
///
/// This utility function wraps HTTP operations with automatic error handling,
/// exiting the program with a formatted error message if the operation fails.
/// It's designed for CLI applications where HTTP errors should terminate execution.
///
/// # Type Parameters
/// * `T` - The return type of the operation
/// * `F` - The operation function type
/// * `Fut` - The future type returned by the operation
///
/// # Arguments
/// * `operation` - The async operation to execute
/// * `context` - Context description for error messages
///
/// # Returns
/// The result of the operation on success (never returns on error)
///
/// # Examples
/// ```no_run
/// use stigmergy::http_utils::{StigmergyClient, execute_or_exit};
///
/// # async {
/// let client = StigmergyClient::new("http://localhost:3000".to_string());
/// let entities: Vec<String> = execute_or_exit(
///     || client.get("entity"),
///     "Failed to fetch entities"
/// ).await;
/// # };
/// ```
pub async fn execute_or_exit<T, F, Fut>(operation: F, context: &str) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn Error>>>,
{
    match operation().await {
        Ok(result) => result,
        Err(e) => cli_utils::exit_with_error(&format!("{}: {}", context, e)),
    }
}
