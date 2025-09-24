#[cfg(test)]
pub mod test_helpers {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::{DataStore, InMemoryDataStore, SaveEntry, SavefileManager};

    /// Creates a test savefile manager with a unique file path based on module and suffix
    pub fn create_test_savefile_manager_with_path(
        module: &str,
        suffix: &str,
    ) -> (Arc<SavefileManager>, PathBuf) {
        use std::process;
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let test_path = PathBuf::from(format!(
            "test_{}_savefile_{}_{}_{}.jsonl",
            module,
            process::id(),
            timestamp,
            suffix
        ));
        let logger = Arc::new(SavefileManager::new(test_path.clone()));
        (logger, test_path)
    }

    /// Creates a test data store instance
    pub fn test_data_store() -> Arc<dyn DataStore> {
        Arc::new(InMemoryDataStore::new())
    }

    /// Reads and parses save entries from a file path
    pub fn load_entries(savefile_path: &std::path::Path) -> Vec<SaveEntry> {
        use std::fs;
        if !savefile_path.exists() {
            return vec![];
        }

        let contents = fs::read_to_string(savefile_path).unwrap_or_default();
        contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<SaveEntry>(line).expect("Failed to parse save entry")
            })
            .collect()
    }

    /// Clears/removes a savefile if it exists
    pub fn clear_savefile(savefile_path: &std::path::Path) {
        use std::fs;
        if savefile_path.exists() {
            fs::remove_file(savefile_path).ok();
        }
    }
}
