use crate::extractor::Task;
use serde::{Deserialize, Serialize};

/// Filter options for task search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOptions {
    pub status: Option<String>,
    pub due_on: Option<String>,
    pub due_before: Option<String>,
    pub due_after: Option<String>,
    pub completed_on: Option<String>,
    pub completed_before: Option<String>,
    pub completed_after: Option<String>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
}

pub fn filter_tasks(tasks: Vec<Task>, options: &FilterOptions) -> Vec<Task> {
    tasks
        .into_iter()
        .filter(|task| {
            // Filter by status
            if let Some(ref status) = options.status
                && &task.status != status
            {
                return false;
            }

            // Filter by exact due date
            if let Some(ref due_on) = options.due_on
                && task.due_date.as_ref() != Some(due_on)
            {
                return false;
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
            if let Some(ref completed_on) = options.completed_on
                && task.completed_date.as_ref() != Some(completed_on)
            {
                return false;
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
            if let Some(ref tags) = options.tags
                && !tags.iter().all(|tag| task.tags.contains(tag))
            {
                return false;
            }

            // Filter by excluded tags (must not have any specified tags)
            if let Some(ref exclude_tags) = options.exclude_tags
                && exclude_tags.iter().any(|tag| task.tags.contains(tag))
            {
                return false;
            }

            true
        })
        .collect()
}
