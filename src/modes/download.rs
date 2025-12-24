use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use colored::*;

use crate::core::Downzer;
use super::{ModeConfig, ModeResult};

pub async fn execute(
    config: ModeConfig,
    downzer: Arc<Downzer>,
    urls: Vec<String>,
    _shutdown: Arc<AtomicBool>,
    task_id: u32,
) -> Result<ModeResult> {
    if !config.quiet {
        println!("{} Modo: Descarga ({} URLs)", "[*]".blue(), urls.len());
        if config.verbose >= 2 {
            println!("  Concurrencia: {}", config.max_concurrent);
            println!("  Timeout: {}s", config.timeout);
            if config.mac.is_some() {
                println!("  MAC Address personalizada: sí");
            }
            if config.ua.is_some() {
                println!("  User-Agent personalizado: sí");
            }
            if config.no_dns {
                println!("  DNS: deshabilitado");
            }
        }
    }

    let content_types = Vec::new(); // El filtrado de content-type se hace en main
    
    let stats = downzer.execute_download_task(
        task_id,
        &config.url_or_target,
        urls.clone(),
        &config.outdir,
        &content_types,
        config.max_concurrent,
        config.verbose,
        false,
    ).await?;

    Ok(ModeResult {
        mode: "download".to_string(),
        total: urls.len(),
        successful: stats.downloaded,
        failed: stats.errors + stats.not_found,
        errors: vec![],
        custom_data: Some(format!(
            "Descargados: {}, Ignorados: {}, No encontrados: {}, Errores: {}, Bytes: {}",
            stats.downloaded, stats.ignored, stats.not_found, stats.errors, stats.total_bytes
        )),
    })
}
