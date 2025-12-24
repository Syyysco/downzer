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

pub fn get_ipc_name() -> Result<interprocess::local_socket::Name<'static>> {
    "downzer_ipc.sock"
        .to_fs_name::<GenericFilePath>()
        .context("Failed to generate socket name")
}

pub fn run_ipc_server(
    downzer: Arc<Downzer>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    let name = get_ipc_name()?;

    // Limpiar socket anterior si existe
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file("/tmp/downzer_ipc.sock");
    }

    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .context("Failed to create IPC listener")?;

    while !shutdown.load(Ordering::SeqCst) {
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
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
                break;
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
