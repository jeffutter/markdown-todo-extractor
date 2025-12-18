use crate::extractor::TaskExtractor;
use crate::filter::{FilterOptions, filter_tasks};
use crate::tag_extractor::TagExtractor;
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
    Tasks {
        /// Path to file or folder to scan
        #[arg(required = true)]
        path: PathBuf,

        /// Filter by task status (incomplete, completed, cancelled)
        #[arg(long)]
        status: Option<String>,

        /// Filter by exact due date (YYYY-MM-DD)
        #[arg(long)]
        due_on: Option<String>,

        /// Filter tasks due before date (YYYY-MM-DD)
        #[arg(long)]
        due_before: Option<String>,

        /// Filter tasks due after date (YYYY-MM-DD)
        #[arg(long)]
        due_after: Option<String>,

        /// Filter tasks completed on a specific date (YYYY-MM-DD)
        #[arg(long)]
        completed_on: Option<String>,

        /// Filter tasks completed before a specific date (YYYY-MM-DD)
        #[arg(long)]
        completed_before: Option<String>,

        /// Filter tasks completed after a specific date (YYYY-MM-DD)
        #[arg(long)]
        completed_after: Option<String>,

        /// Filter by tags (must have all specified tags)
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Exclude tasks with these tags (must not have any)
        #[arg(long, value_delimiter = ',')]
        exclude_tags: Option<Vec<String>>,
    },
    /// Extract all unique tags from markdown files
    Tags {
        /// Path to file or folder to scan
        #[arg(required = true)]
        path: PathBuf,
    },
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Check that only one MCP mode is selected
        if self.mcp_stdio && self.mcp_http {
            return Err("Cannot use both --mcp-stdio and --mcp-http at the same time".to_string());
        }

        // Check that a command is provided in CLI mode
        if !self.mcp_stdio && !self.mcp_http && self.command.is_none() {
            return Err("A subcommand is required. Use 'tasks' or 'tags'.".to_string());
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
        Some(Commands::Tasks {
            path,
            status,
            due_on,
            due_before,
            due_after,
            completed_on,
            completed_before,
            completed_after,
            tags,
            exclude_tags,
        }) => {
            // Create task extractor
            let extractor = TaskExtractor::new();

            // Extract tasks from the given path
            let tasks = extractor.extract_tasks(path)?;

            // Apply filters
            let filter_options = FilterOptions {
                status: status.clone(),
                due_on: due_on.clone(),
                due_before: due_before.clone(),
                due_after: due_after.clone(),
                completed_on: completed_on.clone(),
                completed_before: completed_before.clone(),
                completed_after: completed_after.clone(),
                tags: tags.clone(),
                exclude_tags: exclude_tags.clone(),
            };
            let filtered_tasks = filter_tasks(tasks, &filter_options);

            // Output as JSON
            let json = serde_json::to_string_pretty(&filtered_tasks)?;
            println!("{}", json);

            Ok(())
        }
        Some(Commands::Tags { path }) => {
            // Create tag extractor
            let extractor = TagExtractor::new();

            // Extract tags from the given path
            let tags = extractor.extract_tags(path)?;

            // Output as JSON
            let json = serde_json::to_string_pretty(&tags)?;
            println!("{}", json);

            Ok(())
        }
        None => Err("No command provided".into()),
    }
}
