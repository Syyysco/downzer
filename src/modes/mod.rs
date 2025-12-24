pub mod download;
pub mod webrequest;
pub mod portscan;
pub mod network;

use anyhow::Result;
use std::path::PathBuf;
use crate::core::Downzer;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone)]
pub struct ModeConfig {
    pub mode: String,
    pub url_or_target: String,
    pub method: Option<String>,
    pub data: Option<String>,
    pub data_file: Option<PathBuf>,
    pub download_body: bool,
    pub mac: Option<Vec<String>>,
    pub ua: Option<Vec<String>>,
    pub no_dns: bool,
    pub timeout: u64,
    pub max_concurrent: usize,
    pub verbose: u8,
    pub quiet: bool,
    pub outdir: PathBuf,
    pub proxy: Option<String>,
}

pub async fn execute_mode(
    mode_config: ModeConfig,
    downzer: Arc<Downzer>,
    urls: Vec<String>,
    shutdown: Arc<AtomicBool>,
    task_id: u32,
) -> Result<ModeResult> {
    match mode_config.mode.to_lowercase().as_str() {
        "download" => download::execute(mode_config, downzer, urls, shutdown, task_id).await,
        "webrequest" | "web" => webrequest::execute(mode_config, downzer, urls, shutdown, task_id).await,
        "portscan" | "port" => portscan::execute(mode_config, downzer, urls, shutdown, task_id).await,
        "ssh" | "ftp" | "telnet" | "mail" | "imap" | "pop3" | "smtp" => {
            network::execute(mode_config, downzer, urls, shutdown, task_id).await
        }
        _ => anyhow::bail!("Unknown mode: {}. Available: download, webrequest, portscan, ssh, ftp, telnet, mail", mode_config.mode),
    }
}

#[derive(Debug, Clone)]
pub struct ModeResult {
    pub mode: String,
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub custom_data: Option<String>,
}
