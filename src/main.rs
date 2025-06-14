use tokio::fs;
use tokio::task::JoinSet;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use std::env;
use std::collections::HashMap;

mod vscode_client;
mod web_server;

// Adaptive timing constants for better responsiveness
const MAX_CONCURRENT_TASKS: usize = 50;
const FAST_UPDATE_INTERVAL_SECS: u64 = 1; // When changes detected
const SLOW_UPDATE_INTERVAL_SECS: u64 = 3; // When idle
const PROCESS_CACHE_TTL_SECS: u64 = 1; // Reduced cache TTL
const VSCODE_CHECK_INTERVAL_SECS: u64 = 2; // Much faster VSCode checks
const IDLE_THRESHOLD_COUNT: u32 = 3; // Switch to slow mode after 3 unchanged cycles

#[derive(Debug, Clone)]
pub struct TieredApp {
    name: String,
    tier: u32,
}

#[derive(Debug, Clone)]
pub struct RunningApp {
    name: String,
    tier: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputData {
    pub text: String,
}

// Enhanced cache structure with change detection
#[derive(Debug)]
struct ProcessCache {
    processes: HashMap<String, RunningApp>,
    last_updated: SystemTime,
    last_process_count: usize,
    process_list_hash: u64,
}

impl ProcessCache {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            last_updated: SystemTime::UNIX_EPOCH,
            last_process_count: 0,
            process_list_hash: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.last_updated.elapsed().unwrap_or(Duration::MAX) > Duration::from_secs(PROCESS_CACHE_TTL_SECS)
    }

    // Calculate a simple hash of running process names for change detection
    fn calculate_process_hash(processes: &[RunningApp]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        for app in processes {
            app.name.hash(&mut hasher);
            app.tier.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn has_processes_changed(&self, new_processes: &[RunningApp]) -> bool {
        let new_hash = Self::calculate_process_hash(new_processes);
        let new_count = new_processes.len();
        
        new_hash != self.process_list_hash || new_count != self.last_process_count
    }

    fn update_with_change_detection(&mut self, new_processes: Vec<RunningApp>) -> bool {
        let has_changed = self.has_processes_changed(&new_processes);
        
        self.processes.clear();
        for app in &new_processes {
            self.processes.insert(app.name.clone(), app.clone());
        }
        
        self.last_updated = SystemTime::now();
        self.process_list_hash = Self::calculate_process_hash(&new_processes);
        self.last_process_count = new_processes.len();
        
        has_changed
    }
}

/// Optimized function to get running applications with resource limits and caching
pub async fn get_running_apps_optimized(
    apps_to_check: &[TieredApp],
    cache: &mut ProcessCache
) -> (Vec<RunningApp>, bool) {
    // Return cached results if still valid
    if !cache.is_expired() {
        let cached_results: Vec<RunningApp> = cache.processes.values()
            .filter(|app| apps_to_check.iter().any(|check| app.name.starts_with(&check.name)))
            .cloned()
            .collect();
        return (cached_results, false); // No change, using cache
    }

    let mut running_apps = Vec::new();
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_TASKS));
    
    // Read /proc directory
    let mut proc_dir = match fs::read_dir("/proc").await {
        Ok(dir) => dir,
        Err(_) => return (Vec::new(), false),
    };
    
    let apps_to_check = apps_to_check.to_vec(); // Convert slice to owned vec for move
    
    // Process entries with concurrency limit
    while let Ok(Some(entry)) = proc_dir.next_entry().await {
        let path = entry.path();
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.chars().all(|c| c.is_ascii_digit()) {
                let apps_to_check_clone = apps_to_check.clone();
                let semaphore_clone = semaphore.clone();
                
                tasks.spawn(async move {
                    let _permit = semaphore_clone.acquire().await.ok()?;
                    
                    // Fast path: only read what we need
                    let exe_path = path.join("exe");
                    
                    if let Ok(exe_target) = fs::read_link(&exe_path).await {
                        if let Some(app_name) = exe_target.file_name().and_then(|n| n.to_str()) {
                            // Check if this app matches any from our list
                            for check_app in &apps_to_check_clone {
                                if app_name.starts_with(&check_app.name) {
                                    return Some(RunningApp {
                                        name: app_name.to_string(),
                                        tier: check_app.tier,
                                    });
                                }
                            }
                        }
                    }
                    None
                });
            }
        }
    }
    
    // Collect results with better error handling
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Some(running_app)) => running_apps.push(running_app),
            Ok(None) => continue,
            Err(_) => continue, // Ignore task panics
        }
    }
    
    // Sort by tier only (first come first serve within tier)
    running_apps.sort_by(|a, b| a.tier.cmp(&b.tier));
    
    // Update cache and detect changes
    let has_changed = cache.update_with_change_detection(running_apps.clone());
    
    (running_apps, has_changed)
}

/// Check if VS Code is running (optimized)
fn is_vscode_running(apps: &[RunningApp]) -> bool {
    apps.iter().any(|app| app.name.starts_with("code"))
}

/// Generate text for an application based on its type and context (optimized with string interpolation)
fn generate_app_text(app: &RunningApp, vscode_file_info: Option<&vscode_client::FileInfo>) -> String {
    match app.name.as_str() {
        name if name.starts_with("code") => {
            match vscode_file_info {
                Some(file_info) => format!("editing {} in Visual Studio Code", file_info.file_name),
                None => "VS Code".to_string(),
            }
        }
        name if name.starts_with("zen") => "browsing with Zen browser".to_string(),
        name if name.starts_with("chrome") => "probably on her work account on Chrome".to_string(),
        name if name.starts_with("discord") => "yapping on Discord".to_string(),
        name if name.starts_with("steam") => "gaming on Steam".to_string(),
        name if name.starts_with("vlc") => "watching a movie (will probably log it in letterboxd/bilgi42".to_string(),
        name if name.starts_with("stremio") => "legally streaming some content in stremio".to_string(),
        name if name.starts_with("ghostty") => "using the best terminal emulator (ghostty)".to_string(),
        _ => app.name.clone()
    }
}

/// Optimized presence data updater with adaptive timing and smart change detection
async fn update_presence_data(shared_data: web_server::SharedData, broadcaster: web_server::Broadcaster) {
    let apps_to_check = vec![
        // Tier 1 - The ones you wanna flex the most
        TieredApp { name: "code".to_string(), tier: 1 },
        TieredApp { name: "discord".to_string(), tier: 1 },
        
        // Tier 2 - The apps that you'll use in your off-days (and sometimes on your work days)
        TieredApp { name: "zen".to_string(), tier: 2 },
        TieredApp { name: "chrome".to_string(), tier: 2 },
        TieredApp { name: "steam".to_string(), tier: 2 },
        
        // Tier 3 - Less common applications
        TieredApp { name: "vlc".to_string(), tier: 3 },
        TieredApp { name: "stremio".to_string(), tier: 3 },
        
        // Tier 4 - Terminal emulators
        TieredApp { name: "ghostty".to_string(), tier: 4 },
    ];

    let mut process_cache = ProcessCache::new();
    let mut last_vscode_check = SystemTime::UNIX_EPOCH;
    let mut cached_vscode_info: Option<vscode_client::FileInfo> = None;
    let mut idle_count = 0u32;
    let mut last_output_text = String::new();

    loop {
        let (running_apps, processes_changed) = get_running_apps_optimized(&apps_to_check, &mut process_cache).await;
        
        // Adaptive VSCode checks - faster when VSCode is running
        let mut vscode_file_info: Option<vscode_client::FileInfo> = None;
        
        if is_vscode_running(&running_apps) {
            let should_check_vscode = last_vscode_check.elapsed()
                .unwrap_or(Duration::MAX) > Duration::from_secs(VSCODE_CHECK_INTERVAL_SECS);
            
            if should_check_vscode {
                // Use timeout for VSCode connection to prevent hanging
                match tokio::time::timeout(
                    Duration::from_secs(1), // Reduced timeout for faster response
                    vscode_client::connect_to_vscode_once(3847)
                ).await {
                    Ok(Ok(file_info)) => {
                        cached_vscode_info = Some(file_info.clone());
                        vscode_file_info = Some(file_info);
                        last_vscode_check = SystemTime::now();
                    }
                    Ok(Err(_)) | Err(_) => {
                        // Use cached info if available, otherwise fallback
                        vscode_file_info = cached_vscode_info.clone();
                    }
                }
            } else {
                // Use cached VSCode info
                vscode_file_info = cached_vscode_info.clone();
            }
        } else {
            // Clear cached VSCode info if VSCode is not running
            cached_vscode_info = None;
        }

        // Generate output text for the most relevant application
        let output_text = match running_apps.first() {
            Some(app) => generate_app_text(app, vscode_file_info.as_ref()),
            None => "idle".to_string(),
        };

        // Check if output actually changed
        let output_changed = output_text != last_output_text;
        
        if output_changed {
            let output = OutputData { text: output_text.clone() };
            last_output_text = output_text;
            idle_count = 0; // Reset idle counter on change

            // Update shared data efficiently
            {
                let mut data = shared_data.write().await;
                *data = output.clone();
                
                // Broadcast the change
                let _ = broadcaster.send(output);
            }
        } else if processes_changed {
            // Processes changed but output is the same, reset idle counter
            idle_count = 0;
        } else {
            // No changes detected
            idle_count += 1;
        }

        // Adaptive sleep timing based on activity
        let sleep_duration = if idle_count >= IDLE_THRESHOLD_COUNT {
            Duration::from_secs(SLOW_UPDATE_INTERVAL_SECS) // Slow polling when idle
        } else {
            Duration::from_secs(FAST_UPDATE_INTERVAL_SECS) // Fast polling when active
        };

        tokio::time::sleep(sleep_duration).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    // Get port from environment variable or default to 3001
    let port = env::var("REPRESENCE_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    // Initialize shared data
    let shared_data = Arc::new(RwLock::new(OutputData {
        text: "starting...".to_string(),
    }));

    // Clone shared data for the background task
    let data_for_task = shared_data.clone();

    // Create and start web server
    let (app, broadcaster) = web_server::create_server(shared_data).await;

    // Start background task to update presence data
    tokio::spawn(async move {
        update_presence_data(data_for_task, broadcaster).await;
    });
    
    println!("Represence server starting on http://0.0.0.0:{}", port);
    println!("API endpoint: http://0.0.0.0:{}/api/represence", port);
    println!("Health check: http://0.0.0.0:{}/health", port);
    println!("Optimized for fast response times (1-3s adaptive polling)");

    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}