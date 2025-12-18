use rayon::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Extractor for YAML frontmatter tags
pub struct TagExtractor;

impl TagExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract all unique tags from markdown files in the given path
    pub fn extract_tags(&self, path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let files = if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            self.collect_markdown_files(path)?
        };

        // Use a BTreeSet to automatically sort and deduplicate tags
        let tags: BTreeSet<String> = files
            .par_iter()
            .filter_map(|file_path| self.extract_tags_from_file(file_path).ok())
            .flatten()
            .collect();

        Ok(tags.into_iter().collect())
    }

    /// Recursively collect all markdown files in a directory
    fn collect_markdown_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    files.extend(self.collect_markdown_files(&path)?);
                } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Extract tags from a single markdown file
    fn extract_tags_from_file(
        &self,
        file_path: &Path,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        self.extract_tags_from_content(&content)
    }

    /// Extract tags from markdown content by parsing YAML frontmatter
    fn extract_tags_from_content(
        &self,
        content: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let frontmatter = self.extract_frontmatter(content)?;

        if let Some(fm) = frontmatter {
            self.parse_tags_from_frontmatter(&fm)
        } else {
            Ok(vec![])
        }
    }

    /// Extract YAML frontmatter from markdown content
    /// Frontmatter is expected to be at the start of the file between --- delimiters
    fn extract_frontmatter(
        &self,
        content: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = content.lines().collect();

        // Check if the file starts with ---
        if lines.is_empty() || lines[0].trim() != "---" {
            return Ok(None);
        }

        // Find the closing ---
        let mut end_index = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_index = Some(i);
                break;
            }
        }

        if let Some(end) = end_index {
            let frontmatter_lines = &lines[1..end];
            Ok(Some(frontmatter_lines.join("\n")))
        } else {
            Ok(None)
        }
    }

    /// Parse tags from YAML frontmatter
    fn parse_tags_from_frontmatter(
        &self,
        frontmatter: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Parse YAML frontmatter
        let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter)?;

        // Extract tags field
        if let Some(tags_value) = yaml.get("tags") {
            match tags_value {
                // Handle array of tags
                serde_yaml::Value::Sequence(seq) => {
                    let tags: Vec<String> = seq
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    Ok(tags)
                }
                // Handle single tag as string
                serde_yaml::Value::String(s) => Ok(vec![s.clone()]),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let extractor = TagExtractor::new();

        let content = r#"---
title: My Document
tags:
  - rust
  - programming
---

# Content here
"#;

        let frontmatter = extractor.extract_frontmatter(content).unwrap();
        assert!(frontmatter.is_some());
        assert!(frontmatter.unwrap().contains("tags:"));
    }

    #[test]
    fn test_parse_tags_array() {
        let extractor = TagExtractor::new();

        let frontmatter = r#"title: My Document
tags:
  - rust
  - programming
  - cli
"#;

        let tags = extractor.parse_tags_from_frontmatter(frontmatter).unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        assert!(tags.contains(&"cli".to_string()));
    }

    #[test]
    fn test_parse_tags_single_string() {
        let extractor = TagExtractor::new();

        let frontmatter = r#"title: My Document
tags: single-tag
"#;

        let tags = extractor.parse_tags_from_frontmatter(frontmatter).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "single-tag");
    }

    #[test]
    fn test_extract_tags_from_content() {
        let extractor = TagExtractor::new();

        let content = r#"---
title: My Document
tags:
  - rust
  - programming
---

# My Document

Some content here.
"#;

        let tags = extractor.extract_tags_from_content(content).unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
    }

    #[test]
    fn test_no_frontmatter() {
        let extractor = TagExtractor::new();

        let content = r#"# My Document

Some content here without frontmatter.
"#;

        let tags = extractor.extract_tags_from_content(content).unwrap();
        assert_eq!(tags.len(), 0);
    }
}
