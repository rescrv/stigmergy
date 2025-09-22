use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;

use crate::cli_utils;

#[derive(Debug)]
pub struct HttpError {
    message: String,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for HttpError {}

pub struct StigmergyClient {
    client: Client,
    base_url: String,
}

impl StigmergyClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Constructs a full API URL from a path
    pub fn api_url(&self, path: &str) -> String {
        let path = path.strip_prefix('/').unwrap_or(path);
        format!("{}/api/v1/{}", self.base_url, path)
    }

    /// Makes a GET request and handles the response
    pub async fn get<T>(&self, path: &str) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Makes a POST request with JSON body and handles the response
    pub async fn post<B, T>(&self, path: &str, body: &B) -> Result<T, Box<dyn Error>>
    where
        B: serde::Serialize,
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.post(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    /// Makes a POST request without body and handles the response
    pub async fn post_empty<T>(&self, path: &str) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Makes a PUT request with JSON body and handles the response
    pub async fn put<B, T>(&self, path: &str, body: &B) -> Result<T, Box<dyn Error>>
    where
        B: serde::Serialize,
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self.client.put(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    /// Makes a DELETE request and handles the response (no body expected)
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

    /// Handles HTTP response, deserializing success or returning error
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

/// Execute an HTTP operation and exit on error with formatted message
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
