use crate::capabilities::CapabilityRegistry;
use crate::capabilities::tags::ExtractTagsRequest;
use crate::capabilities::tasks::SearchTasksRequest;
use crate::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

/// Commandline Args
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Start MCP server on stdin/stdout
    #[arg(long)]
    pub mcp_stdio: bool,

    /// Start MCP server on HTTP (default port: 8000)
    #[arg(long)]
    pub mcp_http: bool,

    /// Port for HTTP MCP server (requires --mcp-http)
    #[arg(long, default_value = "8000")]
    pub port: u16,

    /// Path to file or folder to scan (used in MCP server mode as base path)
    #[arg(global = true)]
    pub path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract and filter tasks from markdown files
    Tasks(Box<TasksCommand>),
    /// Extract all unique tags from markdown files
    Tags {
        /// Path to file or folder to scan
        #[arg(required = true)]
        path: PathBuf,
    },
    /// List all tags with document counts
    ListTags(ListTagsCommand),
    /// Search for files by tags
    SearchByTags(SearchByTagsCommand),
    /// List directory tree
    ListFiles(ListFilesCommand),
    /// Read a file
    ReadFile(ReadFileCommand),
}

#[derive(Parser, Debug)]
pub struct TasksCommand {
    /// Path to file or folder to scan
    #[arg(required = true)]
    pub path: PathBuf,

    /// Filter by task status (incomplete, completed, cancelled)
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by exact due date (YYYY-MM-DD)
    #[arg(long)]
    pub due_on: Option<String>,

    /// Filter tasks due before date (YYYY-MM-DD)
    #[arg(long)]
    pub due_before: Option<String>,

    /// Filter tasks due after date (YYYY-MM-DD)
    #[arg(long)]
    pub due_after: Option<String>,

    /// Filter tasks completed on a specific date (YYYY-MM-DD)
    #[arg(long)]
    pub completed_on: Option<String>,

    /// Filter tasks completed before a specific date (YYYY-MM-DD)
    #[arg(long)]
    pub completed_before: Option<String>,

    /// Filter tasks completed after a specific date (YYYY-MM-DD)
    #[arg(long)]
    pub completed_after: Option<String>,

    /// Filter by tags (must have all specified tags)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,

    /// Exclude tasks with these tags (must not have any)
    #[arg(long, value_delimiter = ',')]
    pub exclude_tags: Option<Vec<String>>,
}

#[derive(Parser, Debug)]
pub struct ListTagsCommand {
    /// Path to file or folder to scan
    #[arg(required = true)]
    pub path: PathBuf,

    /// Minimum document count to include a tag
    #[arg(long)]
    pub min_count: Option<usize>,

    /// Maximum number of tags to return
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Parser, Debug)]
pub struct SearchByTagsCommand {
    /// Path to file or folder to scan
    #[arg(required = true)]
    pub path: PathBuf,

    /// Tags to search for
    #[arg(long, value_delimiter = ',', required = true)]
    pub tags: Vec<String>,

    /// If true, file must have ALL tags (AND logic)
    #[arg(long)]
    pub match_all: bool,

    /// Limit the number of files returned
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Parser, Debug)]
pub struct ListFilesCommand {
    /// Path to file or folder to scan
    #[arg(required = true)]
    pub path: PathBuf,

    /// Maximum depth to traverse
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Include file sizes in output
    #[arg(long)]
    pub include_sizes: bool,
}

#[derive(Parser, Debug)]
pub struct ReadFileCommand {
    /// Path to the vault root
    #[arg(required = true)]
    pub vault_path: PathBuf,

    /// Path to the file relative to vault root
    #[arg(required = true)]
    pub file_path: String,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Check that only one MCP mode is selected
        if self.mcp_stdio && self.mcp_http {
            return Err("Cannot use both --mcp-stdio and --mcp-http at the same time".to_string());
        }

        // Check that a command is provided in CLI mode
        if !self.mcp_stdio && !self.mcp_http && self.command.is_none() {
            return Err("A subcommand is required. Use 'tasks', 'tags', 'list-tags', 'search-by-tags', 'list-files', or 'read-file'.".to_string());
        }

        Ok(())
    }

    pub fn get_base_path(&self) -> PathBuf {
        if self.mcp_stdio || self.mcp_http {
            self.path.clone().unwrap_or_else(|| PathBuf::from("."))
        } else {
            // In CLI mode with subcommands, path comes from the subcommand
            PathBuf::from(".")
        }
    }
}

pub fn run_cli(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    match &args.command {
        Some(Commands::Tasks(tasks_cmd)) => {
            // Load configuration from the task path
            let config = Arc::new(Config::load_from_base_path(&tasks_cmd.path));

            // Create capability registry
            let registry = CapabilityRegistry::new(tasks_cmd.path.clone(), config);

            // Build search request from CLI arguments
            let request = SearchTasksRequest {
                status: tasks_cmd.status.clone(),
                due_on: tasks_cmd.due_on.clone(),
                due_before: tasks_cmd.due_before.clone(),
                due_after: tasks_cmd.due_after.clone(),
                completed_on: tasks_cmd.completed_on.clone(),
                completed_before: tasks_cmd.completed_before.clone(),
                completed_after: tasks_cmd.completed_after.clone(),
                tags: tasks_cmd.tags.clone(),
                exclude_tags: tasks_cmd.exclude_tags.clone(),
                limit: None, // No limit for CLI
            };

            // Search tasks using TaskCapability
            let response = registry.tasks().search_tasks_sync(request)?;

            // Output as JSON
            let json = serde_json::to_string_pretty(&response.tasks)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::Tags { path }) => {
            // Load configuration from the path
            let config = Arc::new(Config::load_from_base_path(path));

            // Create capability registry
            let registry = CapabilityRegistry::new(path.clone(), config);

            // Build extract tags request
            let request = ExtractTagsRequest { subpath: None };

            // Extract tags using TagCapability
            let response = registry.tags().extract_tags_sync(request)?;

            // Output as JSON
            let json = serde_json::to_string_pretty(&response.tags)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::ListTags(cmd)) => {
            use crate::capabilities::tags::ListTagsRequest;

            let config = Arc::new(Config::load_from_base_path(&cmd.path));
            let registry = CapabilityRegistry::new(cmd.path.clone(), config);

            let request = ListTagsRequest {
                path: None,
                min_count: cmd.min_count,
                limit: cmd.limit,
            };

            let response = registry.tags().list_tags_sync(request)?;

            let json = serde_json::to_string_pretty(&response)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::SearchByTags(cmd)) => {
            use crate::capabilities::tags::SearchByTagsRequest;

            let config = Arc::new(Config::load_from_base_path(&cmd.path));
            let registry = CapabilityRegistry::new(cmd.path.clone(), config);

            let request = SearchByTagsRequest {
                tags: cmd.tags.clone(),
                match_all: Some(cmd.match_all),
                subpath: None,
                limit: cmd.limit,
            };

            let response = registry.tags().search_by_tags_sync(request)?;

            let json = serde_json::to_string_pretty(&response)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::ListFiles(cmd)) => {
            use crate::capabilities::files::ListFilesRequest;

            let config = Arc::new(Config::load_from_base_path(&cmd.path));
            let registry = CapabilityRegistry::new(cmd.path.clone(), config);

            let request = ListFilesRequest {
                path: None,
                max_depth: cmd.max_depth,
                include_sizes: Some(cmd.include_sizes),
            };

            let response = registry.files().list_files_sync(request)?;

            let json = serde_json::to_string_pretty(&response)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::ReadFile(cmd)) => {
            use crate::capabilities::files::ReadFileRequest;

            let config = Arc::new(Config::load_from_base_path(&cmd.vault_path));
            let registry = CapabilityRegistry::new(cmd.vault_path.clone(), config);

            let request = ReadFileRequest {
                path: cmd.file_path.clone(),
            };

            let response = registry.files().read_file_sync(request)?;

            let json = serde_json::to_string_pretty(&response)?;
            println!("{}", json);

            Ok(())
        }
        None => Err("No command provided".into()),
    }
}
