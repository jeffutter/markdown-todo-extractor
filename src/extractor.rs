use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Represents a task found in a markdown file
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub content: String,
    pub status: String,
    pub file_path: String,
    pub file_name: String,
    pub line_number: usize,
    pub raw_line: String,
    pub tags: Vec<String>,
    pub sub_items: Vec<String>,
    pub summary: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub created_date: Option<String>,
    pub completed_date: Option<String>,
}

/// Extracts tasks from markdown files
pub struct TaskExtractor {
    task_incomplete: Regex,
    task_completed: Regex,
    task_cancelled: Regex,
    task_other: Regex,
    tag_pattern: Regex,
    due_date_patterns: Vec<Regex>,
    priority_pattern: Regex,
    created_patterns: Vec<Regex>,
    completion_patterns: Vec<Regex>,
    // Cleaning patterns (moved from clean_content())
    timestamp_pattern: Regex,
    priority_emoji_pattern: Regex,
    priority_text_pattern: Regex,
    whitespace_pattern: Regex,
    // Sub-item pattern (moved from parse_sub_item())
    checkbox_pattern: Regex,
}

impl TaskExtractor {
    pub fn new() -> Self {
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
            // Cleaning patterns
            timestamp_pattern: Regex::new(r"^\d{2}:\d{2} ").unwrap(),
            priority_emoji_pattern: Regex::new(r"[⏫🔼🔽⏬]").unwrap(),
            priority_text_pattern: Regex::new(r"(?i)priority:\s*(high|medium|low)").unwrap(),
            whitespace_pattern: Regex::new(r"\s+").unwrap(),
            // Sub-item pattern
            checkbox_pattern: Regex::new(r"^-\s*\[.\]\s*(.+)$").unwrap(),
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
        cleaned = self.timestamp_pattern.replace_all(&cleaned, " ").to_string();

        // Remove priority indicators
        cleaned = self.priority_emoji_pattern.replace_all(&cleaned, "").to_string();
        cleaned = self.priority_text_pattern.replace_all(&cleaned, "").to_string();

        // Remove created date patterns
        for pattern in &self.created_patterns {
            cleaned = pattern.replace_all(&cleaned, "").to_string();
        }

        // Remove completed date patterns
        for pattern in &self.completion_patterns {
            cleaned = pattern.replace_all(&cleaned, "").to_string();
        }

        // Clean up extra whitespace
        cleaned = self.whitespace_pattern.replace_all(&cleaned, " ").to_string();
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
            if let Some(caps) = self.checkbox_pattern.captures(stripped) {
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

    pub fn extract_tasks(&self, path: &Path) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
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

impl Default for TaskExtractor {
    fn default() -> Self {
        Self::new()
    }
}
