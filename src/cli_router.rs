use crate::capabilities::CapabilityRegistry;
use std::sync::Arc;

/// Trait for CLI operations that can be automatically registered
#[async_trait::async_trait]
pub trait CliOperation: Send + Sync + 'static {
    /// The CLI command name (e.g., "tasks", "list-tags")
    fn command_name(&self) -> &'static str;

    /// Get the clap command definition (from request struct's Parser derive)
    fn get_command(&self) -> clap::Command;

    /// Execute the operation from parsed CLI arguments
    ///
    /// This method receives:
    /// - matches: The clap ArgMatches for this subcommand
    /// - registry: The capability registry to use
    ///
    /// Returns JSON string for output
    async fn execute_from_args(
        &self,
        matches: &clap::ArgMatches,
        registry: &CapabilityRegistry,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

/// Build a clap Command dynamically from all registered operations
pub fn build_cli(operations: &[Arc<dyn CliOperation>]) -> clap::Command {
    let mut cmd = clap::Command::new("markdown-todo-extractor")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Extract todo items from Markdown files")
        .arg(
            clap::Arg::new("path")
                .long("path")
                .short('p')
                .help("Base path to search (defaults to current directory)")
                .global(true),
        );

    // Add each operation's command definition
    for operation in operations {
        cmd = cmd.subcommand(operation.get_command());
    }

    cmd
}

/// Execute CLI command by routing to the appropriate operation
pub async fn execute_cli(
    operations: &[Arc<dyn CliOperation>],
    matches: clap::ArgMatches,
    registry: &CapabilityRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the matching operation
    if let Some((subcommand_name, sub_matches)) = matches.subcommand() {
        for operation in operations {
            if operation.command_name() == subcommand_name {
                let output = operation.execute_from_args(sub_matches, registry).await?;
                println!("{}", output);
                return Ok(());
            }
        }
        return Err(format!("Unknown command: {}", subcommand_name).into());
    }

    Err("No command specified".into())
}
