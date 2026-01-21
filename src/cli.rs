use clap::Parser;
use std::path::PathBuf;

/// Commandline Args for MCP server modes
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
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Check that only one MCP mode is selected
        if self.mcp_stdio && self.mcp_http {
            return Err("Cannot use both --mcp-stdio and --mcp-http at the same time".to_string());
        }

        // Check that at least one MCP mode is selected
        if !self.mcp_stdio && !self.mcp_http {
            return Err("Must specify either --mcp-stdio or --mcp-http. For CLI commands, use: tasks, tags, list-tags, search-tags, list-files, or read-file.".to_string());
        }

        Ok(())
    }

    pub fn get_base_path(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}
