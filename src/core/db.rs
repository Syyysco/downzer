use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use crate::core::task::TaskStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: u32,
    pub url_template: String,
    pub total: usize,
    pub completed: usize,
    pub status: TaskStatus,
    pub pid: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path();
        let conn = Connection::open(db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY,
                url_template TEXT NOT NULL,
                total INTEGER DEFAULT 0,
                completed INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                pid INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        
        Ok(Self { conn })
    }
    
    fn db_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("downzer");
        std::fs::create_dir_all(&path).ok();
        path.push("tasks.db");
        path
    }
    
    pub fn insert_task(&self, task: &TaskRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tasks (id, url_template, total, completed, status, pid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                task.id,
                task.url_template,
                task.total,
                task.completed,
                task.status.to_string(),
                task.pid,
                task.created_at,
                task.updated_at
            ],
        )?;
        Ok(())
    }
    
    pub fn update_task(&self, task: &TaskRecord) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET total=?1, completed=?2, status=?3, updated_at=?4 WHERE id=?5",
            params![task.total, task.completed, task.status.to_string(), task.updated_at, task.id],
        )?;
        Ok(())
    }
    
    pub fn get_task(&self, id: u32) -> Result<Option<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url_template, total, completed, status, pid, created_at, updated_at 
             FROM tasks WHERE id=?1"
        )?;
        
        let mut rows = stmt.query(params![id])?;
        
        if let Some(row) = rows.next()? {
            let status_str: String = row.get(4)?;
            Ok(Some(TaskRecord {
                id: row.get(0)?,
                url_template: row.get(1)?,
                total: row.get(2)?,
                completed: row.get(3)?,
                status: TaskStatus::from_string(&status_str),
                pid: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }
    
    pub fn get_active_tasks(&self) -> Result<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url_template, total, completed, status, pid, created_at, updated_at 
             FROM tasks WHERE status IN ('Running', 'Paused', 'Queued')"
        )?;
        
        let tasks = stmt.query_map([], |row| {
            let status_str: String = row.get(4)?;
            Ok(TaskRecord {
                id: row.get(0)?,
                url_template: row.get(1)?,
                total: row.get(2)?,
                completed: row.get(3)?,
                status: TaskStatus::from_string(&status_str),
                pid: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        
        let mut result = Vec::new();
        for task in tasks {
            result.push(task?);
        }
        Ok(result)
    }
    
    pub fn delete_task(&self, id: u32) -> Result<()> {
        self.conn.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
        Ok(())
    }
}