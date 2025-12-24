use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use colored::*;

use crate::core::Downzer;
use super::{ModeConfig, ModeResult};

pub async fn execute(
    config: ModeConfig,
    downzer: Arc<Downzer>,
    urls: Vec<String>,
    shutdown: Arc<AtomicBool>,
    _task_id: u32,
) -> Result<ModeResult> {
    if !config.quiet {
        println!("{} Modo: Peticiones Web ({} URLs)", "[*]".blue(), urls.len());
        if config.verbose >= 2 {
            println!("  Método: {}", config.method.as_deref().unwrap_or("GET").green());
            println!("  Concurrencia: {}", config.max_concurrent);
            println!("  Timeout: {}s", config.timeout);
            if config.download_body {
                println!("  Descargar respuesta: sí");
            }
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

    let method = config.method.as_deref().unwrap_or("GET").to_uppercase();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent));
    let mut handles = vec![];
    let mut successful = 0;
    let mut failed = 0;
    let start = Instant::now();

    for (idx, url) in urls.iter().enumerate() {
        // Check for shutdown before spawning each task
        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let sem = semaphore.clone();
        let client = downzer.client.clone();
        let url = url.clone();
        let method = method.clone();
        let verbose = config.verbose;
        let quiet = config.quiet;
        let request_timeout = std::time::Duration::from_secs(config.timeout);

        let handle = tokio::spawn(async move {
            let _guard = sem.acquire().await.ok()?;

            // Add timeout to prevent hanging requests
            let result = match tokio::time::timeout(request_timeout, match method.as_str() {
                "GET" => client.get(&url).send(),
                "POST" => client.post(&url).send(),
                "PUT" => client.put(&url).send(),
                "DELETE" => client.delete(&url).send(),
                "PATCH" => client.patch(&url).send(),
                "HEAD" => client.head(&url).send(),
                "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url).send(),
                _ => return Some((false, 0)),
            }).await {
                Ok(Ok(resp)) => Ok(resp),
                Ok(Err(e)) => Err(e),
                Err(_) => {
                    if verbose >= 1 {
                        eprintln!("  {} {} - {}", format!("[{}]", idx + 1).cyan(), url.red(), "Timeout".red());
                    }
                    return Some((false, 0));
                }
            };

            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let success = resp.status().is_success();
                    
                    if verbose >= 2 {
                        if success {
                            println!("  {} {} [{}]", format!("[{}]", idx + 1).cyan(), url, status.to_string().green());
                        } else {
                            println!("  {} {} [{}]", format!("[{}]", idx + 1).cyan(), url, status.to_string().red());
                        }
                    }
                    
                    Some((success, status))
                }
                Err(e) => {
                    if verbose >= 1 {
                        eprintln!("  {} {} - {}", format!("[{}]", idx + 1).cyan(), url.red(), e.to_string().red());
                    }
                    Some((false, 0))
                }
            }
        });

        handles.push(handle);
    }

    if config.verbose >= 2 && !config.quiet {
        println!("{} Procesando {} peticiones...", "[*]".blue(), urls.len());
    }

    // Procesar resultados - también aquí checar shutdown
    for handle in handles {
        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        
        if let Ok(Some((success, _status))) = handle.await {
            if success {
                successful += 1;
            } else {
                failed += 1;
            }
        } else {
            failed += 1;
        }
    }

    let elapsed = start.elapsed();

    if config.verbose >= 1 || !config.quiet {
        println!();
        println!("{}", "═══════════════════════════════════════".green());
        println!("{} Peticiones completadas en {:.2}s", "[✓]".green(), elapsed.as_secs_f64());
        println!("  Exitosas: {} ({}%)", successful.to_string().green(), 
                 if urls.len() > 0 { (successful * 100 / urls.len()) as u32 } else { 0 });
        println!("  Fallidas: {} ({}%)", failed.to_string().yellow(), 
                 if urls.len() > 0 { (failed * 100 / urls.len()) as u32 } else { 0 });
        println!("  Velocidad: {:.2} req/s", (urls.len() as f64 / elapsed.as_secs_f64()));
        println!("{}", "═══════════════════════════════════════".green());
    }

    Ok(ModeResult {
        mode: "webrequest".to_string(),
        total: urls.len(),
        successful,
        failed,
        errors: vec![],
        custom_data: Some(format!("Velocidad: {:.2} req/s", urls.len() as f64 / elapsed.as_secs_f64())),
    })
}
