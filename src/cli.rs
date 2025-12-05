use crate::extractor::TaskExtractor;
use crate::filter::{FilterOptions, filter_tasks};
use clap::Parser;
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

    /// Path to file or folder to scan (required unless in MCP server mode)
    #[arg(required_unless_present_any = ["mcp_stdio", "mcp_http"])]
    pub path: Option<PathBuf>,

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
        Ok(())
    }

    pub fn get_base_path(&self) -> PathBuf {
        if self.mcp_stdio || self.mcp_http {
            self.path.clone().unwrap_or_else(|| PathBuf::from("."))
        } else {
            self.path.clone().expect("Path is required in CLI mode")
        }
    }

    pub fn to_filter_options(&self) -> FilterOptions {
        FilterOptions {
            status: self.status.clone(),
            due_on: self.due_on.clone(),
            due_before: self.due_before.clone(),
            due_after: self.due_after.clone(),
            completed_on: self.completed_on.clone(),
            completed_before: self.completed_before.clone(),
            completed_after: self.completed_after.clone(),
            tags: self.tags.clone(),
            exclude_tags: self.exclude_tags.clone(),
        }
    }
}

pub fn run_cli(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = args.get_base_path();

    // Create task extractor
    let extractor = TaskExtractor::new();

    // Extract tasks from the given path
    let tasks = extractor.extract_tasks(&base_path)?;

    // Apply filters
    let filter_options = args.to_filter_options();
    let filtered_tasks = filter_tasks(tasks, &filter_options);

    // Output as JSON
    let json = serde_json::to_string_pretty(&filtered_tasks)?;
    println!("{}", json);

    Ok(())
}
