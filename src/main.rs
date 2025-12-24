use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use colored::*;

mod core;
mod ipc;
mod audio;
mod ui;
mod modes;

use crate::core::Downzer;
use crate::core::task::{TaskStatus, TaskInfo};
use crate::ipc::IpcCommand;

#[derive(Parser)]
#[command(name = "downzer")]
#[command(about = "Flexible Resource Fuzzer/Downloader - High Performance Edition", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// URL template with FUZZW1, FUZZW2, ... or FUZZR placeholders
    #[arg(value_name = "URL")]
    url: Option<String>,

    /// Mode: download, webrequest, portscan, ssh, ftp, mail, telnet
    #[arg(short = 'm', long = "mode", default_value = "download")]
    mode: String,

    /// Range to replace FUZZR (e.g., 0-30)
    #[arg(short = 'r', long = "range")]
    range: Option<String>,

    /// Wordlists (strings or file paths). Use + to combine adjacent lists
    #[arg(short = 'w', long = "wordlist", num_args = 1..)]
    wordlist: Vec<String>,

    /// Exclude items (comma or space separated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Option<String>,

    /// Iterate lists/ranges in parallel (synchronized iteration)
    #[arg(long)]
    parallel: bool,

    /// Shuffle the order of combinations
    #[arg(long)]
    random: bool,

    /// Accept only specific Content-Types (comma-separated: image, video, application/pdf, etc.)
    #[arg(short = 'c', long = "content-type")]
    content_type: Option<String>,

    /// Delay: <ms> (milliseconds) or <sec>x<N> (pause every N requests)
    #[arg(short = 'd', long = "delay")]
    delay: Option<String>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Quiet mode (no output)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Output directory
    #[arg(short = 'o', long = "outdir", default_value = ".")]
    outdir: PathBuf,

    /// Enable logging
    #[arg(long)]
    log: bool,

    /// Log directory (defaults to output directory)
    #[arg(long = "log-dir")]
    log_dir: Option<PathBuf>,

    /// Debug mode
    #[arg(long)]
    debug: bool,

    /// Proxy URL (http://host:port or socks5://host:port)
    #[arg(long)]
    proxy: Option<String>,

    /// Maximum concurrent connections
    #[arg(long, default_value = "20")]
    max_concurrent: usize,

    /// Add task (non-blocking, runs in background)
    #[arg(long)]
    add: bool,

    /// Add to queue (waits for other tasks to complete)
    #[arg(long)]
    queue: bool,

    /// Timeout per request in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// HTTP method for web requests (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
    #[arg(long)]
    method: Option<String>,

    /// Data to send in request body (for POST, PUT, PATCH)
    #[arg(long)]
    data: Option<String>,

    /// File containing data to send in request body
    #[arg(long)]
    data_file: Option<PathBuf>,

    /// Download response body (--dd or -dd)
    #[arg(long = "dd", alias = "download-body")]
    download_body: bool,

    /// Randomize MAC address
    #[arg(long)]
    random_mac: bool,

    /// Custom MAC address or file with MAC addresses
    #[arg(long)]
    mac: Option<String>,

    /// Randomize User-Agent
    #[arg(long)]
    random_ua: bool,

    /// Custom User-Agent or file with User-Agents
    #[arg(long)]
    ua: Option<String>,

    /// Disable DNS resolution
    #[arg(short = 'n', long = "nodns")]
    no_dns: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Stop running tasks by ID
    Stop {
        /// Task IDs to stop
        ids: Vec<u32>,
    },
    /// List active tasks
    List,
    /// Pause tasks by ID
    Pause {
        ids: Vec<u32>,
    },
    /// Resume paused tasks by ID
    Resume {
        ids: Vec<u32>,
    },
    /// Configuration panel
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle subcommands
    if let Some(command) = &cli.command {
        match command {
            Commands::Stop { ids } => {
                match ipc::send_command(&IpcCommand::Stop(ids.clone())) {
                    Ok(ipc::IpcResponse::Ok) => println!("{} Tasks stopped", "✓".green()),
                    Ok(ipc::IpcResponse::Error(e)) => println!("{} {}", "✗".red(), e),
                    Ok(_) => {}
                    Err(_) => println!("{} No running instance found", "⚠".yellow()),
                }
                return Ok(());
            }
            Commands::List => {
                match ipc::send_command(&IpcCommand::List) {
                    Ok(ipc::IpcResponse::TaskList(tasks)) => {
                        if tasks.is_empty() {
                            println!("No active tasks");
                        } else {
                            println!("{}", "ID\tStatus\tURL".cyan());
                            for (id, status, url) in tasks {
                                println!("{}\t{}\t{}", id, status, url);
                            }
                        }
                    }
                    Err(_) => println!("{} No running instance found", "⚠".yellow()),
                    _ => {}
                }
                return Ok(());
            }
            Commands::Pause { ids } => {
                match ipc::send_command(&IpcCommand::Pause(ids.clone())) {
                    Ok(ipc::IpcResponse::Ok) => println!("{} Tasks paused", "✓".green()),
                    Ok(ipc::IpcResponse::Error(e)) => println!("{} {}", "✗".red(), e),
                    Err(_) => println!("{} No running instance found", "⚠".yellow()),
                    _ => {}
                }
                return Ok(());
            }
            Commands::Resume { ids } => {
                match ipc::send_command(&IpcCommand::Resume(ids.clone())) {
                    Ok(ipc::IpcResponse::Ok) => println!("{} Tasks resumed", "✓".green()),
                    Ok(ipc::IpcResponse::Error(e)) => println!("{} {}", "✗".red(), e),
                    Err(_) => println!("{} No running instance found", "⚠".yellow()),
                    _ => {}
                }
                return Ok(());
            }
            Commands::Config => {
                let mut config = Downzer::load_config();
                if ui::config_ui::show_config_panel(&mut config)? {
                    Downzer::save_config(&config)?;
                    println!("{}", "✓ Configuration saved!".green());
                } else {
                    println!("{}", "Configuration not saved".yellow());
                }
                return Ok(());
            }
        }
    }

    if cli.url.is_none() {
        eprintln!("{} URL template is required", "[ERROR]".red());
        std::process::exit(1);
    }

    let url_template = cli.url.clone().unwrap();

    if !cli.quiet {
        println!("{}", "╔════════════════════════════════════════╗".cyan());
        println!("{}", "║    Downzer - Resource Fuzzer/Download ║".cyan());
        println!("{}", "╚════════════════════════════════════════╝".cyan());
    }

    // Procesar range
    let mut all_items = Vec::new();
    
    if let Some(range_spec) = &cli.range {
        if !cli.quiet {
            println!("{} Processing range: {}", "[*]".blue(), range_spec);
        }
        let range_items = Downzer::parse_range(range_spec).await?;
        all_items.push(range_items);
    }

    // Procesar wordlists
    if !cli.wordlist.is_empty() {
        if !cli.quiet {
            println!("{} Processing {} wordlist(s)", "[*]".blue(), cli.wordlist.len());
        }
        for (idx, wl) in cli.wordlist.iter().enumerate() {
            let items = Downzer::parse_wordlist(wl).await?;
            if cli.verbose >= 1 {
                println!("  [{}] Loaded {} items", idx + 1, items.len());
            }
            all_items.push(items);
        }
    }

    if all_items.is_empty() {
        anyhow::bail!("No wordlists or range specified. Use -r or -w options.");
    }

    // Generar combinaciones
    if !cli.quiet {
        println!("{} Generating combinations...", "[*]".blue());
    }
    
    let combinations = Downzer::generate_combinations(&all_items, cli.parallel, cli.random);
    if cli.verbose >= 1 {
        println!("  Total combinations: {}", combinations.len());
    }

    // Procesar template de URL
    if !cli.quiet {
        println!("{} Processing URL template", "[*]".blue());
    }
    
    let urls = Downzer::process_url_template(&url_template, combinations, cli.exclude.as_deref())?;
    
    if cli.verbose >= 1 {
        println!("  Total URLs to download: {}", urls.len());
    }

    if urls.is_empty() {
        anyhow::bail!("No URLs generated after filtering");
    }

    // Parse content types
    let content_types: Vec<String> = cli.content_type
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // IPC shared state
    let shutdown = Arc::new(AtomicBool::new(false));

    // Setup Ctrl+C handler using tokio's signal handling
    let shutdown_signal = shutdown.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown_signal.store(true, Ordering::SeqCst);
    });

    // Initialize Downzer
    if cli.verbose >= 1 {
        println!("{} Initializing Downzer", "[*]".blue());
    }
    
    let downzer = Downzer::new(cli.proxy.clone(), cli.timeout).await?;

    // Start IPC server in background only if not running in quick mode
    // IPC server is blocking, so only start it if we expect interactive use
    if cli.add || cli.queue {
        let downzer_ipc = downzer.clone();
        let shutdown_ipc = shutdown.clone();
        std::thread::spawn(move || {
            // Ignorar errores de IPC, no es crítico
            let _ = ipc::run_ipc_server(downzer_ipc, shutdown_ipc);
        });
    }

    // Get next task ID
    let task_id = {
        let mut next_id = downzer.next_task_id.write().await;
        let id = *next_id;
        *next_id += 1;
        id
    };

    // Create task info
    let task_info = TaskInfo {
        id: task_id,
        url_template: url_template.clone(),
        total: urls.len(),
        completed: 0,
        status: TaskStatus::Running,
        start_time: Instant::now(),
    };

    downzer.add_task(task_info).await;

    if !cli.quiet {
        println!("{} Task #{} started", "[✓]".green(), task_id);
        println!("{} {} URLs to download from {}", "[*]".blue(), urls.len(), url_template);
        println!();
    }

    // Parse MAC addresses
    let mac_list = if let Some(mac_str) = &cli.mac {
        Downzer::parse_wordlist(mac_str).await?
    } else {
        vec![]
    };

    // Parse User-Agents
    let ua_list = if let Some(ua_str) = &cli.ua {
        Downzer::parse_wordlist(ua_str).await?
    } else {
        vec![]
    };

    // Create mode configuration
    let mode_config = modes::ModeConfig {
        mode: cli.mode.clone(),
        url_or_target: url_template.clone(),
        method: cli.method.clone(),
        data: cli.data.clone(),
        data_file: cli.data_file.clone(),
        download_body: cli.download_body,
        mac: if mac_list.is_empty() { None } else { Some(mac_list) },
        ua: if ua_list.is_empty() { None } else { Some(ua_list) },
        no_dns: cli.no_dns,
        timeout: cli.timeout,
        max_concurrent: cli.max_concurrent,
        verbose: cli.verbose,
        quiet: cli.quiet,
        outdir: cli.outdir.clone(),
        proxy: cli.proxy.clone(),
    };

    // Spawn mode executor task with shutdown support
    let downzer_worker = downzer.clone();
    let shutdown_worker = shutdown.clone();
    let urls_copy = urls.clone();
    let quiet = cli.quiet;
    let verbose = cli.verbose;

    let executor_handle = tokio::spawn(async move {
        match modes::execute_mode(
            mode_config,
            downzer_worker.clone(),
            urls_copy,
            shutdown_worker.clone(),
            task_id,
        ).await {
            Ok(result) => {
                if verbose >= 1 || !quiet {
                    println!("\n{}", "═══════════════════════════════════════".green());
                    println!("{} Task #{} completed", "[✓]".green(), task_id);
                    println!("  Mode: {} ({})", result.mode, result.total);
                    println!("  Successful: {}", result.successful);
                    println!("  Failed:     {}", result.failed);
                    if !result.errors.is_empty() && verbose >= 2 {
                        println!("  Errors:");
                        for err in &result.errors {
                            println!("    - {}", err);
                        }
                    }
                    if let Some(custom) = &result.custom_data {
                        println!("  Details: {}", custom);
                    }
                    println!("{}", "═══════════════════════════════════════".green());
                }
                shutdown_worker.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                eprintln!("{} Task #{} failed: {}", "[✗]".red(), task_id, e);
                shutdown_worker.store(true, Ordering::SeqCst);
            }
        }
    });

    // Wait for executor to complete
    let _ = executor_handle.await;

    // Cleanup
    println!("{} Limpiando...", "[*]".blue());
    shutdown.store(true, Ordering::SeqCst);
    
    // Wait a moment for tasks to cleanup
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Cleanup socket files
    let _ = ipc::cleanup_old_sockets();

    if !cli.quiet {
        println!("{} Done!", "[✓]".green());
    }

    Ok(())
}