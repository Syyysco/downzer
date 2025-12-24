use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

use crate::core::downzer::Downzer;
use crate::core::task::TaskStatus;

pub async fn run_task(
    downzer: Arc<Downzer>,
    task_id: u32,
) -> anyhow::Result<()> {
    // Obtener info de la tarea
    let _task_info = downzer.get_task_info(task_id).await;
    
    loop {
        match downzer.get_task_status(task_id).await {
            Some(TaskStatus::Paused) => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            Some(TaskStatus::Stopped) | Some(TaskStatus::Completed) | None => break,
            Some(TaskStatus::Running) => {}
            _ => break,
        }

        do_work_step(downzer.clone(), task_id).await?;
    }

    Ok(())
}

async fn do_work_step(
    _downzer: Arc<Downzer>,
    _task_id: u32,
) -> Result<()> {
    // Simular un paso de trabajo (descarga de un archivo)
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}