//! Error types for stigmergy operations.

/// Errors that can occur during data store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataStoreError {
    /// The requested item was not found in the data store.
    NotFound,
    /// An item with the same identifier already exists.
    AlreadyExists,
    /// JSON serialization or deserialization failed.
    SerializationError(String),
    /// An I/O operation failed (for persistent storage backends).
    IoError(String),
    /// An internal storage system error occurred.
    Internal(String),
}

impl std::fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Item not found in data store"),
            Self::AlreadyExists => write!(f, "Item already exists in data store"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<sqlx::Error> for DataStoreError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => DataStoreError::NotFound,
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                DataStoreError::AlreadyExists
            }
            _ => DataStoreError::Internal(e.to_string()),
        }
    }
}

impl std::error::Error for DataStoreError {}
