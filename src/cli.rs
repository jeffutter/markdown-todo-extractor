use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
        None => Err("No command provided".into()),
    }
}
