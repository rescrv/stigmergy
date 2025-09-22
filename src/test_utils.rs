#[cfg(test)]
pub mod test_helpers {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::{DataStore, DurableLogger, InMemoryDataStore, LogEntry};

    /// Creates a test logger with a unique file path based on module and suffix
    pub fn create_test_logger_with_path(
        module: &str,
        suffix: &str,
    ) -> (Arc<DurableLogger>, PathBuf) {
        use std::process;
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let test_path = PathBuf::from(format!(
            "test_{}_logging_{}_{}_{}.jsonl",
            module,
            process::id(),
            timestamp,
            suffix
        ));
        let logger = Arc::new(DurableLogger::new(test_path.clone()));
        (logger, test_path)
    }

    /// Creates a test data store instance
    pub fn test_data_store() -> Arc<dyn DataStore> {
        Arc::new(InMemoryDataStore::new())
    }

    /// Reads and parses log entries from a file path
    pub fn read_log_entries(log_path: &std::path::Path) -> Vec<LogEntry> {
        use std::fs;
        if !log_path.exists() {
            return vec![];
        }

        let contents = fs::read_to_string(log_path).unwrap_or_default();
        contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<LogEntry>(line).expect("Failed to parse log entry"))
            .collect()
    }

    /// Clears/removes a log file if it exists
    pub fn clear_log_file(log_path: &std::path::Path) {
        use std::fs;
        if log_path.exists() {
            fs::remove_file(log_path).ok();
        }
    }
}
