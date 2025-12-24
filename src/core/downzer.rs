use regex::Regex;
use reqwest::{Client, Proxy};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::core::task::{TaskInfo, TaskStatus};
use crate::core::db::Database;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub sound_enabled: bool,
    pub sound_min_duration: u64,
    pub sound_volume: f32,
    pub sound_on_task_complete: bool,
    pub sound_on_all_complete: bool,
    pub sound_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            sound_min_duration: 10,
            sound_volume: 0.5,
            sound_on_task_complete: false,
            sound_on_all_complete: true,
            sound_type: "woodensaw".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub downloaded: usize,
    pub total_bytes: u64,
    pub ignored: usize,
    pub errors: usize,
    pub not_found: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            downloaded: 0,
            total_bytes: 0,
            ignored: 0,
            errors: 0,
            not_found: 0,
        }
    }
}

pub struct Downzer {
    pub client: Client,
    pub config: Arc<RwLock<Config>>,
    pub tasks: Arc<RwLock<HashMap<u32, TaskInfo>>>,
    pub next_task_id: Arc<RwLock<u32>>,
    pub db: Arc<tokio::sync::Mutex<Database>>,
}

impl Downzer {
    pub async fn new(proxy: Option<String>, timeout: u64) -> anyhow::Result<Arc<Self>> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .gzip(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

        if let Some(proxy_url) = proxy {
            let proxy = Proxy::all(&proxy_url)?;
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder.build()?;
        let config = Self::load_config();
        let db = Database::new()?;

        Ok(Arc::new(Self {
            client,
            config: Arc::new(RwLock::new(config)),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            next_task_id: Arc::new(RwLock::new(1)),
            db: Arc::new(tokio::sync::Mutex::new(db)),
        }))
    }

    pub fn load_config() -> Config {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Config::default()
    }

    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("downzer");
        fs::create_dir_all(&path).ok();
        path.push("config.json");
        path
    }

    pub fn save_config(config: &Config) -> anyhow::Result<()> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(config)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub async fn parse_range(spec: &str) -> anyhow::Result<Vec<String>> {
        let re = Regex::new(r"^(\d+)-(\d+)$")?;
        if let Some(caps) = re.captures(spec) {
            let start: usize = caps[1].parse()?;
            let end: usize = caps[2].parse()?;
            if end < start {
                anyhow::bail!("Invalid range: end < start");
            }
            Ok((start..=end).map(|n| n.to_string()).collect())
        } else {
            anyhow::bail!("Invalid range format: {}. Expected: start-end", spec);
        }
    }

    pub async fn parse_wordlist(token: &str) -> anyhow::Result<Vec<String>> {
        Self::read_list_from_token(token)
    }

    fn read_list_from_token(token: &str) -> anyhow::Result<Vec<String>> {
        if token == "+" {
            return Ok(vec!["+".to_string()]);
        }

        let path = Path::new(token);
        if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let items: Vec<String> = reader
                .lines()
                .filter_map(|l| l.ok())
                .flat_map(|l| l.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(items)
        } else {
            Ok(token.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        }
    }

    fn process_wordlists(tokens: &[String]) -> anyhow::Result<Vec<Vec<String>>> {
        let mut raw_lists = Vec::new();
        for token in tokens {
            let list = Self::read_list_from_token(token)?;
            raw_lists.push(list);
        }

        let mut processed = Vec::new();
        let mut i = 0;

        while i < raw_lists.len() {
            if raw_lists[i].len() == 1 && raw_lists[i][0] == "+" {
                anyhow::bail!("'+' without adjacent lists");
            }

            let mut combined = raw_lists[i].clone();
            let mut j = i + 1;

            while j < raw_lists.len() && raw_lists[j].len() == 1 && raw_lists[j][0] == "+" {
                if j + 1 >= raw_lists.len() {
                    anyhow::bail!("'+' at end without following list");
                }
                combined.extend(raw_lists[j + 1].clone());
                j += 2;
            }

            processed.push(combined);
            i = j;
        }

        Ok(processed)
    }

    pub fn generate_combinations(
        lists: &[Vec<String>],
        parallel: bool,
        random: bool,
    ) -> Vec<Vec<String>> {
        if parallel {
            let len = lists[0].len();
            let mut combinations = Vec::new();
            for i in 0..len {
                let mut combo = Vec::new();
                for list in lists {
                    combo.push(list[i % list.len()].clone());
                }
                combinations.push(combo);
            }
            if random {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                combinations.shuffle(&mut rng);
            }
            combinations
        } else if lists.len() == 1 {
            let mut combinations: Vec<Vec<String>> = lists[0].iter().map(|s| vec![s.clone()]).collect();
            if random {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                combinations.shuffle(&mut rng);
            }
            combinations
        } else {
            fn cartesian_product(lists: &[Vec<String>], current: Vec<String>, index: usize, result: &mut Vec<Vec<String>>) {
                if index >= lists.len() {
                    result.push(current);
                    return;
                }
                for item in &lists[index] {
                    let mut new_current = current.clone();
                    new_current.push(item.clone());
                    cartesian_product(lists, new_current, index + 1, result);
                }
            }

            let mut combinations = Vec::new();
            cartesian_product(lists, Vec::new(), 0, &mut combinations);
            
            if random {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                combinations.shuffle(&mut rng);
            }
            combinations
        }
    }

    pub async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        content_types: &[String],
        verbose: u8,
        debug: bool,
    ) -> anyhow::Result<(u64, String, u16)> {
        if debug {
            println!("[DEBUG] Downloading: {}", url);
        }

        let response = self.client.get(url).send().await?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(anyhow::anyhow!("NOT_FOUND"));
        }

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP {}", status));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if !content_types.is_empty() {
            let matches = content_types.iter().any(|ct| content_type.contains(ct));
            if !matches {
                return Err(anyhow::anyhow!("IGNORED"));
            }
        }

        let content_length = response.content_length().unwrap_or(0);
        let bytes = response.bytes().await?;

        fs::create_dir_all(dest.parent().unwrap())?;
        let mut file = File::create(dest)?;
        file.write_all(&bytes)?;

        if verbose >= 2 {
            println!("[OK] {} ({} bytes)", dest.display(), bytes.len());
        }

        Ok((content_length, content_type, status))
    }

    pub async fn get_task_status(&self, task_id: u32) -> Option<TaskStatus> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).map(|t| t.status)
    }

    pub async fn set_task_status(&self, task_id: u32, status: TaskStatus) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = status;
        }
    }

    pub async fn add_task(&self, task: TaskInfo) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id, task);
    }

    pub async fn update_task_progress(&self, task_id: u32, completed: usize) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.completed = completed;
        }
    }

    pub async fn get_task_info(&self, task_id: u32) -> Option<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).cloned()
    }

    pub fn process_url_template(
        template: &str,
        combinations: Vec<Vec<String>>,
        exclude: Option<&str>,
    ) -> anyhow::Result<Vec<String>> {
        let mut urls = Vec::new();
        let exclude_set: std::collections::HashSet<_> = exclude
            .unwrap_or("")
            .split(|c| c == ',' || c == ' ')
            .filter(|s| !s.is_empty())
            .collect();

        for combo in combinations {
            let mut url = template.to_string();
            
            // Reemplazar FUZZW1, FUZZW2, etc
            for (i, value) in combo.iter().enumerate() {
                let placeholder = format!("FUZZW{}", i + 1);
                if url.contains(&placeholder) {
                    url = url.replace(&placeholder, value);
                }
            }

            // Reemplazar FUZZR si existe
            if url.contains("FUZZR") && !combo.is_empty() {
                url = url.replace("FUZZR", &combo[0]);
            }

            if !exclude_set.contains(url.as_str()) {
                urls.push(url);
            }
        }

        Ok(urls)
    }

    pub async fn execute_download_task(
        &self,
        task_id: u32,
        _url_template: &str,
        urls: Vec<String>,
        output_dir: &Path,
        content_types: &[String],
        max_concurrent: usize,
        verbose: u8,
        debug: bool,
    ) -> anyhow::Result<Stats> {
        let mut stats = Stats::new();

        // Usar un semáforo para limitar concurrencia
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let mut handles = vec![];

        for (idx, url) in urls.iter().enumerate() {
            let url = url.clone();
            let sem = semaphore.clone();
            let output_dir = output_dir.to_path_buf();
            let content_types = content_types.to_vec();
            let self_client = self.client.clone();
            let self_tasks = self.tasks.clone();
            let self_config = self.config.clone();
            let self_next_id = self.next_task_id.clone();
            let self_db = self.db.clone();

            let handle = tokio::spawn(async move {
                let _guard = sem.acquire().await.ok()?;
                
                // Verificar si la tarea fue pausada/detenida
                let tasks_lock = self_tasks.read().await;
                if let Some(task) = tasks_lock.get(&task_id) {
                    if task.status == TaskStatus::Stopped {
                        return None;
                    }
                }
                drop(tasks_lock);

                // Generar nombre de archivo
                let filename = format!("download_{:06}", idx);
                let dest = output_dir.join(&filename);

                // Crear cliente temporal para descarga
                let downzer_temp = Downzer {
                    client: self_client,
                    config: self_config,
                    tasks: self_tasks.clone(),
                    next_task_id: self_next_id,
                    db: self_db,
                };

                // Intentar descarga
                match downzer_temp.download_file(&url, &dest, &content_types, verbose, debug).await {
                    Ok((size, _, _)) => {
                        let mut tasks_mut = self_tasks.write().await;
                        if let Some(t) = tasks_mut.get_mut(&task_id) {
                            t.completed += 1;
                        }
                        Some((size, 1, 0, 0, 0))
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        let mut tasks_mut = self_tasks.write().await;
                        if let Some(t) = tasks_mut.get_mut(&task_id) {
                            t.completed += 1;
                        }
                        
                        if err_msg.contains("NOT_FOUND") {
                            Some((0, 0, 1, 0, 1))
                        } else if err_msg.contains("IGNORED") {
                            Some((0, 0, 1, 0, 0))
                        } else {
                            if verbose >= 1 {
                                eprintln!("[ERROR] {}: {}", url, err_msg);
                            }
                            Some((0, 0, 0, 1, 0))
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Esperar a que todas las tareas terminen
        for handle in handles {
            if let Ok(Some((bytes, downloaded, ignored, errors, not_found))) = handle.await {
                stats.total_bytes += bytes;
                stats.downloaded += downloaded;
                stats.ignored += ignored;
                stats.errors += errors;
                stats.not_found += not_found;
            }
        }

        // Marcar tarea como completada
        self.set_task_status(task_id, TaskStatus::Completed).await;

        if verbose >= 1 {
            println!("[SUMMARY]");
            println!("  Downloaded: {}", stats.downloaded);
            println!("  Ignored: {}", stats.ignored);
            println!("  Not Found: {}", stats.not_found);
            println!("  Errors: {}", stats.errors);
            println!("  Total bytes: {}", stats.total_bytes);
        }

        Ok(stats)
    }
}