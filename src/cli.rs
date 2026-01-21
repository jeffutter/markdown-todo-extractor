use crate::capabilities::CapabilityRegistry;
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
            return Err("A subcommand is required. Use 'tasks', 'tags', 'list-tags', 'search-tags', 'list-files', or 'read-file'.".to_string());
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
    // Manual routing for commands that haven't been migrated to CLI router yet
    match &args.command {
        Some(Commands::Tasks(_)) => {
            // This should be handled by the CLI router in main.rs
            unreachable!("Tasks command should be handled by CLI router")
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
