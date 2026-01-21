pub mod files;
pub mod tags;
pub mod tasks;

use crate::config::Config;
use rmcp::model::ErrorData;
use std::path::PathBuf;
use std::sync::Arc;

use self::files::FileCapability;
use self::tags::TagCapability;
use self::tasks::TaskCapability;

/// Result type for capability operations
pub type CapabilityResult<T> = Result<T, ErrorData>;

/// Registry for managing capabilities
///
/// This registry holds all capabilities and provides getter methods for
/// accessing them. All capabilities are initialized at startup.
pub struct CapabilityRegistry {
    // Capability instances
    task_capability: Arc<TaskCapability>,
    tag_capability: Arc<TagCapability>,
    file_capability: Arc<FileCapability>,
}

impl CapabilityRegistry {
    /// Create a new capability registry with all capabilities initialized
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        Self {
            task_capability: Arc::new(TaskCapability::new(base_path.clone(), Arc::clone(&config))),
            tag_capability: Arc::new(TagCapability::new(base_path.clone(), Arc::clone(&config))),
            file_capability: Arc::new(FileCapability::new(base_path, config)),
        }
    }

    /// Get the task capability
    pub fn tasks(&self) -> Arc<TaskCapability> {
        Arc::clone(&self.task_capability)
    }

    /// Get the tag capability
    pub fn tags(&self) -> Arc<TagCapability> {
        Arc::clone(&self.tag_capability)
    }

    /// Get the file capability
    pub fn files(&self) -> Arc<FileCapability> {
        Arc::clone(&self.file_capability)
    }

    /// Create all operations for automatic registration
    ///
    /// This is the single source of truth for which operations are exposed via HTTP, CLI, and MCP.
    /// Each operation wraps a capability method and implements the unified Operation trait.
    pub fn create_operations(&self) -> Vec<Arc<dyn crate::operation::Operation>> {
        vec![
            // Task operations
            Arc::new(tasks::SearchTasksOperation::new(self.tasks())),
            // Tag operations
            Arc::new(tags::ExtractTagsOperation::new(self.tags())),
            Arc::new(tags::ListTagsOperation::new(self.tags())),
            Arc::new(tags::SearchByTagsOperation::new(self.tags())),
            // File operations
            Arc::new(files::ListFilesOperation::new(self.files())),
            Arc::new(files::ReadFileOperation::new(self.files())),
        ]
    }
}
