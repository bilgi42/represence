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

// Constants for optimization
const MAX_CONCURRENT_TASKS: usize = 50;
const UPDATE_INTERVAL_SECS: u64 = 5;
const PROCESS_CACHE_TTL_SECS: u64 = 2; // Cache process list for 2 seconds

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

// Cache structure for process information
#[derive(Debug)]
struct ProcessCache {
    processes: HashMap<String, RunningApp>,
    last_updated: SystemTime,
}

impl ProcessCache {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            last_updated: SystemTime::UNIX_EPOCH,
        }
    }

    fn is_expired(&self) -> bool {
        self.last_updated.elapsed().unwrap_or(Duration::MAX) > Duration::from_secs(PROCESS_CACHE_TTL_SECS)
    }
}

/// Optimized function to get running applications with resource limits and caching
pub async fn get_running_apps_optimized(
    apps_to_check: &[TieredApp],
    cache: &mut ProcessCache
) -> Vec<RunningApp> {
    // Return cached results if still valid
    if !cache.is_expired() {
        return cache.processes.values()
            .filter(|app| apps_to_check.iter().any(|check| app.name.starts_with(&check.name)))
            .cloned()
            .collect();
    }

    let mut running_apps = Vec::new();
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_TASKS));
    
    // Read /proc directory
    let mut proc_dir = match fs::read_dir("/proc").await {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
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
    
    // Update cache
    cache.processes.clear();
    for app in &running_apps {
        cache.processes.insert(app.name.clone(), app.clone());
    }
    cache.last_updated = SystemTime::now();
    
    // Sort by tier only (first come first serve within tier)
    running_apps.sort_by(|a, b| a.tier.cmp(&b.tier));
    
    running_apps
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

/// Optimized presence data updater with better resource management
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

    loop {
        let running_apps = get_running_apps_optimized(&apps_to_check, &mut process_cache).await;
        
        // Optimize VSCode checks - only check if VSCode is running and cache is old
        let mut vscode_file_info: Option<vscode_client::FileInfo> = None;
        
        if is_vscode_running(&running_apps) {
            let should_check_vscode = last_vscode_check.elapsed()
                .unwrap_or(Duration::MAX) > Duration::from_secs(10); // Check VSCode every 10 seconds max
            
            if should_check_vscode {
                let vscode_port: u16 = env::var("REPRESENCE_VSCODE_PORT")
                    .unwrap_or_else(|_| "3847".to_string())
                    .parse()
                    .unwrap_or(3847);
                
                // Use timeout for VSCode connection to prevent hanging
                match tokio::time::timeout(
                    Duration::from_secs(2),
                    vscode_client::connect_to_vscode_once(vscode_port)
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

        let output = OutputData { text: output_text };

        // Update shared data efficiently
        {
            let mut data = shared_data.write().await;
            if data.text != output.text {  // Only update if changed
                *data = output.clone();
                
                // Only broadcast if data actually changed
                let _ = broadcaster.send(output);
            }
        }

        // Use tokio::time::sleep for better resource management
        tokio::time::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
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
    
    println!("Represence server starting on http://0.0.0.0:3001");
    println!("API endpoint: http://0.0.0.0:3001/api/represence");
    println!("Health check: http://0.0.0.0:3001/health");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}