use serde::{Serialize, Deserialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Stopped, // añadido para IPC explícito
}

impl TaskStatus {
    pub fn to_string(&self) -> String {
        match self {
            TaskStatus::Queued => "Queued".to_string(),
            TaskStatus::Running => "Running".to_string(),
            TaskStatus::Paused => "Paused".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::Failed => "Failed".to_string(),
            TaskStatus::Stopped => "Stopped".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "Queued" => TaskStatus::Queued,
            "Running" => TaskStatus::Running,
            "Paused" => TaskStatus::Paused,
            "Completed" => TaskStatus::Completed,
            "Failed" => TaskStatus::Failed,
            "Stopped" => TaskStatus::Stopped,
            _ => TaskStatus::Queued,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: u32,
    pub url_template: String,
    pub total: usize,
    pub completed: usize,
    pub status: TaskStatus,
    pub start_time: Instant,
}