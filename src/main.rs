use clap::Parser;
use regex::Regex;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Represents a task found in a markdown file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    content: String,
    status: String,
    file_path: String,
    file_name: String,
    line_number: usize,
    raw_line: String,
    tags: Vec<String>,
    sub_items: Vec<String>,
    summary: Option<String>,
    due_date: Option<String>,
    priority: Option<String>,
    created_date: Option<String>,
    completed_date: Option<String>,
}

/// Extracts tasks from markdown files
struct TaskExtractor {
    task_incomplete: Regex,
    task_completed: Regex,
    task_cancelled: Regex,
    task_other: Regex,
    tag_pattern: Regex,
    due_date_patterns: Vec<Regex>,
    priority_pattern: Regex,
    created_patterns: Vec<Regex>,
    completion_patterns: Vec<Regex>,
}

impl TaskExtractor {
    fn new() -> Self {
        TaskExtractor {
            task_incomplete: Regex::new(r"^(\s*)-\s*\[\s\]\s*(.+)$").unwrap(),
            task_completed: Regex::new(r"(?i)^(\s*)-\s*\[x\]\s*(.+)$").unwrap(),
            task_cancelled: Regex::new(r"^(\s*)-\s*\[-\]\s*(.+)$").unwrap(),
            task_other: Regex::new(r"^(\s*)-\s*\[(.)\]\s*(.+)$").unwrap(),
            tag_pattern: Regex::new(r"#(\w+)").unwrap(),
            due_date_patterns: vec![
                Regex::new(r"📅\s*(\d{4}-\d{2}-\d{2})").unwrap(),
                Regex::new(r"due:\s*(\d{4}-\d{2}-\d{2})").unwrap(),
                Regex::new(r"@due\((\d{4}-\d{2}-\d{2})\)").unwrap(),
            ],
            priority_pattern: Regex::new(r"[⏫🔼🔽⏬]|priority:\s*(high|medium|low)").unwrap(),
            created_patterns: vec![
                Regex::new(r"➕\s*(\d{4}-\d{2}-\d{2})").unwrap(),
                Regex::new(r"created:\s*(\d{4}-\d{2}-\d{2})").unwrap(),
            ],
            completion_patterns: vec![
                Regex::new(r"✅\s*(\d{4}-\d{2}-\d{2})").unwrap(),
                Regex::new(r"completed:\s*(\d{4}-\d{2}-\d{2})").unwrap(),
            ],
        }
    }

    fn extract_tags(&self, content: &str) -> Vec<String> {
        self.tag_pattern
            .captures_iter(content)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }

    fn extract_due_date(&self, content: &str) -> Option<String> {
        for pattern in &self.due_date_patterns {
            if let Some(caps) = pattern.captures(content) {
                return Some(caps.get(1).unwrap().as_str().to_string());
            }
        }
        None
    }

    fn extract_priority(&self, content: &str) -> Option<String> {
        if let Some(caps) = self.priority_pattern.captures(content) {
            if content.contains("⏫") {
                return Some("urgent".to_string());
            } else if content.contains("🔼") {
                return Some("high".to_string());
            } else if content.contains("🔽") {
                return Some("low".to_string());
            } else if content.contains("⏬") {
                return Some("lowest".to_string());
            } else if let Some(priority_text) = caps.get(1) {
                return Some(priority_text.as_str().to_lowercase());
            }
        }
        None
    }

    fn extract_created_date(&self, content: &str) -> Option<String> {
        for pattern in &self.created_patterns {
            if let Some(caps) = pattern.captures(content) {
                return Some(caps.get(1).unwrap().as_str().to_string());
            }
        }
        None
    }

    fn extract_completed_date(&self, content: &str) -> Option<String> {
        for pattern in &self.completion_patterns {
            if let Some(caps) = pattern.captures(content) {
                return Some(caps.get(1).unwrap().as_str().to_string());
            }
        }
        None
    }

    fn clean_content(&self, content: &str) -> String {
        let mut cleaned = content.to_string();

        // Remove due date patterns
        for pattern in &self.due_date_patterns {
            cleaned = pattern.replace_all(&cleaned, "").to_string();
        }

        // Remove timestamp prefix
        let timestamp_pattern = Regex::new(r"^\d{2}:\d{2} ").unwrap();
        cleaned = timestamp_pattern.replace_all(&cleaned, " ").to_string();

        // Remove priority indicators
        let priority_emoji_pattern = Regex::new(r"[⏫🔼🔽⏬]").unwrap();
        cleaned = priority_emoji_pattern.replace_all(&cleaned, "").to_string();

        let priority_text_pattern = Regex::new(r"(?i)priority:\s*(high|medium|low)").unwrap();
        cleaned = priority_text_pattern.replace_all(&cleaned, "").to_string();

        // Remove created date patterns
        for pattern in &self.created_patterns {
            cleaned = pattern.replace_all(&cleaned, "").to_string();
        }

        // Remove completed date patterns
        for pattern in &self.completion_patterns {
            cleaned = pattern.replace_all(&cleaned, "").to_string();
        }

        // Clean up extra whitespace
        let whitespace_pattern = Regex::new(r"\s+").unwrap();
        cleaned = whitespace_pattern.replace_all(&cleaned, " ").to_string();
        cleaned = cleaned.trim().to_string();

        cleaned
    }

    fn is_sub_item(&self, line: &str, parent_line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Get indentation levels
        let parent_indent = parent_line.len() - parent_line.trim_start().len();
        let line_indent = line.len() - line.trim_start().len();

        // Sub-item must be more indented than parent
        if line_indent <= parent_indent {
            return false;
        }

        // Check if it's a list item (starts with - or *)
        let stripped = line.trim_start();
        stripped.starts_with('-')
            || stripped.starts_with('*')
            || stripped.starts_with("- [")
            || stripped.starts_with("* [")
    }

    fn parse_sub_item(&self, line: &str) -> Option<String> {
        let stripped = line.trim();

        // Handle checkbox sub-items
        if stripped.starts_with("- [") {
            let checkbox_pattern = Regex::new(r"^-\s*\[.\]\s*(.+)$").unwrap();
            if let Some(caps) = checkbox_pattern.captures(stripped) {
                return Some(caps.get(1).unwrap().as_str().trim().to_string());
            }
        }

        // Handle regular list items
        if stripped.starts_with('-') || stripped.starts_with('*') {
            return Some(stripped[1..].trim().to_string());
        }

        None
    }

    fn extract_tasks_from_file(
        &self,
        file_path: &Path,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut tasks = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if let Some(mut task) = self.parse_task_line(line, file_path, i + 1) {
                // Look for sub-items on subsequent lines
                i += 1;
                while i < lines.len() {
                    let next_line = lines[i];
                    if self.is_sub_item(next_line, &task.raw_line) {
                        if let Some(sub_item) = self.parse_sub_item(next_line) {
                            task.sub_items.push(sub_item);
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }
                tasks.push(task);
            } else {
                i += 1;
            }
        }

        Ok(tasks)
    }

    fn extract_tasks(&self, path: &Path) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut all_tasks = Vec::new();

        if path.is_file() {
            // Single file
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                all_tasks.extend(self.extract_tasks_from_file(path)?);
            }
        } else if path.is_dir() {
            // Directory - recursively find all .md files
            self.extract_tasks_from_dir(path, &mut all_tasks)?;
        } else {
            return Err(format!("Path does not exist: {}", path.display()).into());
        }

        Ok(all_tasks)
    }

    fn extract_tasks_from_dir(
        &self,
        dir: &Path,
        tasks: &mut Vec<Task>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    match self.extract_tasks_from_file(&path) {
                        Ok(file_tasks) => tasks.extend(file_tasks),
                        Err(e) => eprintln!("Warning: Could not read {:?}: {}", path, e),
                    }
                }
            } else if path.is_dir() {
                self.extract_tasks_from_dir(&path, tasks)?;
            }
        }

        Ok(())
    }

    fn parse_task_line(&self, line: &str, file_path: &Path, line_number: usize) -> Option<Task> {
        let line = line.trim_end_matches(&['\n', '\r'][..]);

        // Try incomplete pattern
        if let Some(caps) = self.task_incomplete.captures(line) {
            let content = caps.get(2).unwrap().as_str().to_string();
            return Some(self.create_task(
                content,
                "incomplete".to_string(),
                line,
                file_path,
                line_number,
            ));
        }

        // Try completed pattern
        if let Some(caps) = self.task_completed.captures(line) {
            let content = caps.get(2).unwrap().as_str().to_string();
            return Some(self.create_task(
                content,
                "completed".to_string(),
                line,
                file_path,
                line_number,
            ));
        }

        // Try cancelled pattern
        if let Some(caps) = self.task_cancelled.captures(line) {
            let content = caps.get(2).unwrap().as_str().to_string();
            return Some(self.create_task(
                content,
                "cancelled".to_string(),
                line,
                file_path,
                line_number,
            ));
        }

        // Try other pattern
        if let Some(caps) = self.task_other.captures(line) {
            let char = caps.get(2).unwrap().as_str();
            let content = caps.get(3).unwrap().as_str().to_string();

            // Skip if it matches standard patterns
            if char == "x" || char == "X" || char == " " || char == "-" {
                return None;
            }

            return Some(self.create_task(
                content,
                format!("other_{}", char),
                line,
                file_path,
                line_number,
            ));
        }

        None
    }

    fn create_task(
        &self,
        content: String,
        status: String,
        raw_line: &str,
        file_path: &Path,
        line_number: usize,
    ) -> Task {
        // Extract metadata from content
        let tags = self.extract_tags(&content);
        let due_date = self.extract_due_date(&content);
        let priority = self.extract_priority(&content);
        let created_date = self.extract_created_date(&content);
        let completed_date = self.extract_completed_date(&content);

        // Clean content by removing metadata
        let clean_content = self.clean_content(&content);

        Task {
            content: clean_content,
            status,
            file_path: file_path.to_string_lossy().to_string(),
            file_name: file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            line_number,
            raw_line: raw_line.to_string(),
            tags,
            sub_items: Vec::new(),
            summary: None,
            due_date,
            priority,
            created_date,
            completed_date,
        }
    }
}

/// Commandline Args
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start MCP server on stdin/stdout
    #[arg(long)]
    mcp: bool,

    /// Path to file or folder to scan (required in both MCP and CLI modes)
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
}

/// Filter options for task search
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterOptions {
    status: Option<String>,
    due_on: Option<String>,
    due_before: Option<String>,
    due_after: Option<String>,
    completed_on: Option<String>,
    completed_before: Option<String>,
    completed_after: Option<String>,
    tags: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
}

fn filter_tasks_with_options(tasks: Vec<Task>, options: &FilterOptions) -> Vec<Task> {
    tasks
        .into_iter()
        .filter(|task| {
            // Filter by status
            if let Some(ref status) = options.status {
                if &task.status != status {
                    return false;
                }
            }

            // Filter by exact due date
            if let Some(ref due_on) = options.due_on {
                if task.due_date.as_ref() != Some(due_on) {
                    return false;
                }
            }

            // Filter by due before date
            if let Some(ref due_before) = options.due_before {
                if let Some(ref due_date) = task.due_date {
                    if due_date >= due_before {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by due after date
            if let Some(ref due_after) = options.due_after {
                if let Some(ref due_date) = task.due_date {
                    if due_date <= due_after {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by exact completed date
            if let Some(ref completed_on) = options.completed_on {
                if task.completed_date.as_ref() != Some(completed_on) {
                    return false;
                }
            }

            // Filter by completed before date
            if let Some(ref completed_before) = options.completed_before {
                if let Some(ref completed_date) = task.completed_date {
                    if completed_date >= completed_before {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by completed after date
            if let Some(ref completed_after) = options.completed_after {
                if let Some(ref completed_date) = task.completed_date {
                    if completed_date <= completed_after {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by tags (must have all specified tags)
            if let Some(ref tags) = options.tags {
                if !tags.iter().all(|tag| task.tags.contains(tag)) {
                    return false;
                }
            }

            // Filter by excluded tags (must not have any specified tags)
            if let Some(ref exclude_tags) = options.exclude_tags {
                if exclude_tags.iter().any(|tag| task.tags.contains(tag)) {
                    return false;
                }
            }

            true
        })
        .collect()
}

fn filter_tasks(tasks: Vec<Task>, args: &Args) -> Vec<Task> {
    let options = FilterOptions {
        status: args.status.clone(),
        due_on: args.due_on.clone(),
        due_before: args.due_before.clone(),
        due_after: args.due_after.clone(),
        completed_on: args.completed_on.clone(),
        completed_before: args.completed_before.clone(),
        completed_after: args.completed_after.clone(),
        tags: args.tags.clone(),
        exclude_tags: args.exclude_tags.clone(),
    };
    filter_tasks_with_options(tasks, &options)
}

/// MCP Service for task searching
#[derive(Debug, Clone)]
struct TaskSearchService {
    tool_router: ToolRouter<TaskSearchService>,
    base_path: PathBuf,
}

/// Parameters for the search_tasks tool
#[derive(Debug, Deserialize, JsonSchema)]
struct SearchTasksRequest {
    #[schemars(description = "Filter by task status (incomplete, completed, cancelled)")]
    status: Option<String>,

    #[schemars(description = "Filter by exact due date (YYYY-MM-DD)")]
    due_on: Option<String>,

    #[schemars(description = "Filter tasks due before date (YYYY-MM-DD)")]
    due_before: Option<String>,

    #[schemars(description = "Filter tasks due after date (YYYY-MM-DD)")]
    due_after: Option<String>,

    #[schemars(description = "Filter tasks completed on a specific date (YYYY-MM-DD)")]
    completed_on: Option<String>,

    #[schemars(description = "Filter tasks completed before a specific date (YYYY-MM-DD)")]
    completed_before: Option<String>,

    #[schemars(description = "Filter tasks completed after a specific date (YYYY-MM-DD)")]
    completed_after: Option<String>,

    #[schemars(description = "Filter by tags (must have all specified tags)")]
    tags: Option<Vec<String>>,

    #[schemars(description = "Exclude tasks with these tags (must not have any)")]
    exclude_tags: Option<Vec<String>>,
}

#[tool_router]
impl TaskSearchService {
    fn new(base_path: PathBuf) -> Self {
        Self {
            tool_router: Self::tool_router(),
            base_path,
        }
    }

    #[tool(
        description = "Search for tasks in Markdown files with optional filtering by status, dates, and tags"
    )]
    async fn search_tasks(
        &self,
        Parameters(request): Parameters<SearchTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Create task extractor
        let extractor = TaskExtractor::new();

        // Extract tasks from the base path
        let tasks = extractor
            .extract_tasks(&self.base_path)
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to extract tasks: {}", e)),
                data: None,
            })?;

        // Apply filters
        let filter_options = FilterOptions {
            status: request.status,
            due_on: request.due_on,
            due_before: request.due_before,
            due_after: request.due_after,
            completed_on: request.completed_on,
            completed_before: request.completed_before,
            completed_after: request.completed_after,
            tags: request.tags,
            exclude_tags: request.exclude_tags,
        };
        let filtered_tasks = filter_tasks_with_options(tasks, &filter_options);

        // Convert to JSON
        let json = serde_json::to_string_pretty(&filtered_tasks).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize tasks: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for TaskSearchService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "A Markdown task extraction service that searches Markdown files for todo items and extracts metadata including tags, dates, priorities, and completion status. Supports filtering by status, due dates, completion dates, and tags."
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // If MCP mode is enabled, start the MCP server
    if args.mcp {
        let service = TaskSearchService::new(args.path).serve(stdio()).await?;
        service.waiting().await?;
        return Ok(());
    }

    // Normal CLI mode
    // Create task extractor
    let extractor = TaskExtractor::new();

    // Extract tasks from the given path
    let tasks = extractor.extract_tasks(&args.path)?;

    // Apply filters
    let filtered_tasks = filter_tasks(tasks, &args);

    // Output as JSON
    let json = serde_json::to_string_pretty(&filtered_tasks)?;
    println!("{}", json);

    Ok(())
}
