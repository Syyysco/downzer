use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    GenericFilePath,
    ListenerOptions,
    ToFsName,
};
use std::{
    io::{BufRead, BufReader, Write},
    sync::{
        Arc, 
        atomic::{AtomicBool, Ordering},
    },
    thread,
    path::PathBuf,
};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use crate::core::downzer::Downzer;
use crate::core::task::TaskStatus;

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcCommand {
    Stop(Vec<u32>),
    Pause(Vec<u32>),
    Resume(Vec<u32>),
    List,
    Status(u32),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Ok,
    TaskList(Vec<(u32, String, String)>),
    Error(String),
}

pub fn get_socket_path() -> PathBuf {
    #[cfg(unix)]
    {
        let mut path = PathBuf::from("/tmp");
        path.push("downzer_ipc.sock");
        path
    }
    
    #[cfg(windows)]
    {
        // En Windows, usar un nombre abstracto que interprocess maneja automáticamente
        let mut path = std::env::temp_dir();
        path.push("downzer_ipc.sock");
        path
    }
}

pub fn cleanup_old_sockets() -> Result<()> {
    let socket_path = get_socket_path();
    
    // Intentar remover socket antigua si existe
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).ok();
    }
    
    Ok(())
}

pub fn get_ipc_name() -> Result<interprocess::local_socket::Name<'static>> {
    let path_str = get_socket_path()
        .to_string_lossy()
        .to_string();
    
    path_str
        .to_fs_name::<GenericFilePath>()
        .context("Failed to generate socket name")
}

pub fn run_ipc_server(
    downzer: Arc<Downzer>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    // Limpiar socket antigua
    cleanup_old_sockets()?;
    
    let name = get_ipc_name()?;

    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .context("Failed to create IPC listener")?;

    // Check shutdown frequently even if no connections
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        match listener.accept() {
            Ok(conn) => {
                let downzer = downzer.clone();
                let shutdown = shutdown.clone();

                thread::spawn(move || {
                    if let Err(e) = handle_client(conn, downzer, shutdown) {
                        eprintln!("IPC error: {e}");
                    }
                });
            }
            Err(_e) => {
                // Accept failed or no connection, try again after a brief sleep
                thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    Ok(())
}

pub fn send_command(cmd: &IpcCommand) -> Result<IpcResponse> {
    let name = get_ipc_name()?;
    let mut stream = LocalSocketStream::connect(name)
        .context("Could not connect to IPC server. Is Downzer running?")?;
    
    let json = serde_json::to_string(cmd)?;
    writeln!(stream, "{}", json)?;
    stream.flush()?;
    
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    
    let resp: IpcResponse = serde_json::from_str(&response)?;
    Ok(resp)
}

fn handle_client(
    mut conn: LocalSocketStream,
    downzer: Arc<Downzer>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    let mut reader = BufReader::new(&conn);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let cmd: IpcCommand = serde_json::from_str(&line)?;
    let response = handle_command(cmd, downzer, shutdown);

    let json = serde_json::to_string(&response)?;
    writeln!(conn, "{json}")?;
    conn.flush()?;

    Ok(())
}

fn handle_command(
    cmd: IpcCommand,
    downzer: Arc<Downzer>,
    _shutdown: Arc<AtomicBool>,
) -> IpcResponse {
    match cmd {
        IpcCommand::Stop(ids) => {
            let tasks = downzer.tasks.blocking_write();
            let mut task_map = tasks;
            
            for id in ids {
                if let Some(task) = task_map.get_mut(&id) {
                    task.status = TaskStatus::Stopped;
                }
            }
            IpcResponse::Ok
        }

        IpcCommand::Pause(ids) => {
            let tasks = downzer.tasks.blocking_write();
            let mut task_map = tasks;
            
            for id in ids {
                if let Some(task) = task_map.get_mut(&id) {
                    task.status = TaskStatus::Paused;
                }
            }
            IpcResponse::Ok
        }

        IpcCommand::Resume(ids) => {
            let tasks = downzer.tasks.blocking_write();
            let mut task_map = tasks;
            
            for id in ids {
                if let Some(task) = task_map.get_mut(&id) {
                    task.status = TaskStatus::Running;
                }
            }
            IpcResponse::Ok
        }

        IpcCommand::List => {
            let tasks = downzer.tasks.blocking_read();
            let list: Vec<_> = tasks
                .iter()
                .map(|(id, task)| (*id, task.status.to_string(), task.url_template.clone()))
                .collect();
            IpcResponse::TaskList(list)
        }

        IpcCommand::Status(id) => {
            let tasks = downzer.tasks.blocking_read();
            if let Some(task) = tasks.get(&id) {
                IpcResponse::TaskList(vec![(id, task.status.to_string(), task.url_template.clone())])
            } else {
                IpcResponse::Error(format!("Task {} not found", id))
            }
        }
    }
}
