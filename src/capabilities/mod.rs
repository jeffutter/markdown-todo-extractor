pub mod daily_notes;
pub mod files;
pub mod tags;
pub mod tasks;

use crate::config::Config;
use rmcp::model::ErrorData;
use std::path::PathBuf;
use std::sync::Arc;

use self::daily_notes::DailyNoteCapability;
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
    daily_note_capability: Arc<DailyNoteCapability>,
}

impl CapabilityRegistry {
    /// Create a new capability registry with all capabilities initialized
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        let file_capability = Arc::new(FileCapability::new(base_path.clone(), Arc::clone(&config)));
        let daily_note_capability = Arc::new(DailyNoteCapability::new(
            base_path.clone(),
            Arc::clone(&config),
            Arc::clone(&file_capability),
        ));

        Self {
            task_capability: Arc::new(TaskCapability::new(base_path.clone(), Arc::clone(&config))),
            tag_capability: Arc::new(TagCapability::new(base_path, Arc::clone(&config))),
            file_capability,
            daily_note_capability,
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

    /// Get the daily note capability
    pub fn daily_notes(&self) -> Arc<DailyNoteCapability> {
        Arc::clone(&self.daily_note_capability)
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
            Arc::new(files::ReadFilesOperation::new(self.files())),
            // Daily note operations
            Arc::new(daily_notes::GetDailyNoteOperation::new(self.daily_notes())),
            Arc::new(daily_notes::SearchDailyNotesOperation::new(
                self.daily_notes(),
            )),
        ]
    }
}
