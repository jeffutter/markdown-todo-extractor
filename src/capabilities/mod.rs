pub mod files;
pub mod tags;
pub mod tasks;

use crate::config::Config;
use rmcp::model::ErrorData;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use self::files::FileCapability;
use self::tags::TagCapability;
use self::tasks::TaskCapability;

/// Result type for capability operations
pub type CapabilityResult<T> = Result<T, ErrorData>;

/// Trait for capabilities that can be exposed via multiple interfaces (MCP, HTTP, CLI)
#[allow(dead_code)]
pub trait Capability: Send + Sync + 'static {
    /// Unique identifier for this capability
    fn id(&self) -> &'static str;

    /// Human-readable description of what this capability provides
    fn description(&self) -> &'static str;
}

/// Registry for managing capabilities with lazy initialization
///
/// This registry holds all capabilities and provides getter methods that
/// lazily initialize capabilities on first access. This avoids creating
/// unused capabilities and maintains efficiency.
pub struct CapabilityRegistry {
    config: Arc<Config>,
    base_path: PathBuf,

    // Capability instances (lazily initialized)
    task_capability: OnceLock<Arc<TaskCapability>>,
    tag_capability: OnceLock<Arc<TagCapability>>,
    file_capability: OnceLock<Arc<FileCapability>>,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        Self {
            config,
            base_path,
            task_capability: OnceLock::new(),
            tag_capability: OnceLock::new(),
            file_capability: OnceLock::new(),
        }
    }

    /// Get the task capability (lazily initialized)
    pub fn tasks(&self) -> Arc<TaskCapability> {
        self.task_capability
            .get_or_init(|| {
                Arc::new(TaskCapability::new(
                    self.base_path.clone(),
                    Arc::clone(&self.config),
                ))
            })
            .clone()
    }

    /// Get the tag capability (lazily initialized)
    pub fn tags(&self) -> Arc<TagCapability> {
        self.tag_capability
            .get_or_init(|| {
                Arc::new(TagCapability::new(
                    self.base_path.clone(),
                    Arc::clone(&self.config),
                ))
            })
            .clone()
    }

    /// Get the file capability (lazily initialized)
    pub fn files(&self) -> Arc<FileCapability> {
        self.file_capability
            .get_or_init(|| {
                Arc::new(FileCapability::new(
                    self.base_path.clone(),
                    Arc::clone(&self.config),
                ))
            })
            .clone()
    }

    /// Get the base path
    #[allow(dead_code)]
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Get the config
    #[allow(dead_code)]
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Create all HTTP operations for automatic registration
    ///
    /// This is the single source of truth for which operations are exposed via HTTP.
    /// Each operation wraps a capability method and implements the HttpOperation trait.
    pub fn create_http_operations(&self) -> Vec<Arc<dyn crate::http_router::HttpOperation>> {
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
