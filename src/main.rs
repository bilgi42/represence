use tokio::fs;
use tokio::task::JoinSet;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use std::env;

mod vscode_client;
mod web_server;

#[derive(Debug, Clone)]
pub struct TieredApp {
    name: String,
    tier: u32,
}

#[derive(Debug)]
pub struct RunningApp {
    name: String,
    tier: u32,
    start_time: SystemTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputData {
    pub text: String,
}

/// Asynchronously checks for running applications from a given list, organized by tiers
/// 
/// # Arguments
/// * `apps_to_check` - Vector of tiered applications to look for
/// 
/// # Returns
/// * `Vec<RunningApp>` - Vector of currently running applications, sorted by tier and start time
pub async fn get_running_apps(apps_to_check: Vec<TieredApp>) -> Vec<RunningApp> {
    let mut running_apps = Vec::new();
    let mut tasks = JoinSet::new();
    
    // Read /proc directory
    let mut proc_dir = match fs::read_dir("/proc").await {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
    };
    
    // Collect all PID directories first
    let mut pid_dirs = Vec::new();
    while let Ok(Some(entry)) = proc_dir.next_entry().await {
        let path = entry.path();
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.chars().all(|c| c.is_ascii_digit()) {
                pid_dirs.push(path);
            }
        }
    }
    
    // Process each PID directory concurrently
    for pid_path in pid_dirs {
        let apps_to_check_clone = apps_to_check.clone();
        tasks.spawn(async move {
            let exe_path = pid_path.join("exe");
            let stat_path = pid_path.join("stat");
            
            // Try to read the exe symlink and stat file
            if let (Ok(exe_target), Ok(stat_content)) = (
                fs::read_link(&exe_path).await,
                fs::read_to_string(&stat_path).await
            ) {
                if let Some(app_name) = exe_target.file_name().and_then(|n| n.to_str()) {
                    // Get process start time from stat file
                    let start_time = if let Some(start_time_str) = stat_content.split_whitespace().nth(21) {
                        if let Ok(start_ticks) = start_time_str.parse::<u64>() {
                            // Convert Linux jiffies to SystemTime
                            let boot_time = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() - (start_ticks / 100);
                            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(boot_time)
                        } else {
                            SystemTime::now()
                        }
                    } else {
                        SystemTime::now()
                    };

                    // Check if this app matches any from our list
                    for check_app in &apps_to_check_clone {
                        if app_name.starts_with(&check_app.name) {
                            return Some(RunningApp {
                                name: app_name.to_string(),
                                tier: check_app.tier,
                                start_time,
                            });
                        }
                    }
                }
            }
            None
        });
    }
    
    // Collect results from all tasks
    while let Some(result) = tasks.join_next().await {
        if let Ok(Some(running_app)) = result {
            running_apps.push(running_app);
        }
    }
    
    // Sort by tier first, then by start time (newest first)
    running_apps.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| b.start_time.cmp(&a.start_time))
    });
    
    running_apps
}

/// Alternative implementation with better error handling and concurrency control
pub async fn get_running_apps_with_limit(
    apps_to_check: Vec<TieredApp>, 
    max_concurrent_tasks: usize
) -> Vec<RunningApp> {
    use tokio::sync::Semaphore;
    use std::sync::Arc;
    
    let mut running_apps = Vec::new();
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));
    
    // Read /proc directory
    let mut proc_dir = match fs::read_dir("/proc").await {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
    };
    
    // Process entries with concurrency limit
    while let Ok(Some(entry)) = proc_dir.next_entry().await {
        let path = entry.path();
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.chars().all(|c| c.is_ascii_digit()) {
                let apps_to_check_clone = apps_to_check.clone();
                let semaphore_clone = semaphore.clone();
                
                tasks.spawn(async move {
                    let _permit = semaphore_clone.acquire().await.ok()?;
                    
                    let exe_path = path.join("exe");
                    let stat_path = path.join("stat");
                    
                    if let (Ok(exe_target), Ok(stat_content)) = (
                        fs::read_link(&exe_path).await,
                        fs::read_to_string(&stat_path).await
                    ) {
                        if let Some(app_name) = exe_target.file_name().and_then(|n| n.to_str()) {
                            // Get process start time from stat file
                            let start_time = if let Some(start_time_str) = stat_content.split_whitespace().nth(21) {
                                if let Ok(start_ticks) = start_time_str.parse::<u64>() {
                                    let boot_time = SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() - (start_ticks / 100);
                                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(boot_time)
                                } else {
                                    SystemTime::now()
                                }
                            } else {
                                SystemTime::now()
                            };

                            for check_app in &apps_to_check_clone {
                                if app_name.starts_with(&check_app.name) {
                                    return Some(RunningApp {
                                        name: app_name.to_string(),
                                        tier: check_app.tier,
                                        start_time,
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
    
    // Collect results
    while let Some(result) = tasks.join_next().await {
        if let Ok(Some(running_app)) = result {
            running_apps.push(running_app);
        }
    }
    
    // Sort by tier first, then by start time (newest first)
    running_apps.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| b.start_time.cmp(&a.start_time))
    });
    
    running_apps
}

/// Check if VS Code is running and return true if found
/// You can also add extra functionalities for other applications, I found it somewhat hard to do so for applications that don't have extensions unfortunately. 
async fn is_vscode_running(apps: &[RunningApp]) -> bool {
    apps.iter().any(|app| app.name.starts_with("code"))
}

/// Generate text for an application based on its type and context
fn generate_app_text(app: &RunningApp, vscode_file_info: Option<&vscode_client::FileInfo>) -> String {
    match app.name.as_str() {
        name if name.starts_with("code") => {
            if let Some(file_info) = vscode_file_info {
                format!("editing {} in VSCode", file_info.file_name)
            } else {
                "VS Code".to_string()
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

/// Update presence data continuously, so everyone can know you're learning Haskell for the 1000th time
async fn update_presence_data(shared_data: web_server::SharedData) {
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
        
        // Tier 4 - Terminal emulators, if you use anything other than ghostty, ngmi
        TieredApp { name: "ghostty".to_string(), tier: 4 },

    ];

    loop {
        let running_apps = get_running_apps(apps_to_check.clone()).await;
        
        // Check if VS Code is running and try to get file information
        let mut vscode_file_info: Option<vscode_client::FileInfo> = None;
        
        if is_vscode_running(&running_apps).await {
            // Get VS Code port from environment variable or use default
            let vscode_port: u16 = env::var("REPRESENCE_VSCODE_PORT")
                .unwrap_or_else(|_| "3847".to_string())
                .parse()
                .unwrap_or(3847);
            
            // Try to connect and get current file info
            match vscode_client::connect_to_vscode_once(vscode_port).await {
                Ok(file_info) => {
                    println!("VSCode connection successful: editing {}", file_info.file_name);
                    vscode_file_info = Some(file_info);
                }
                Err(e) => {
                    println!("VSCode connection failed: {}. Using fallback text.", e);
                    // If we can't connect, VS Code might not have the extension installed
                }
            }
        }

        // Generate output text for the most relevant application (highest tier, most recent)
        let output_text = if let Some(app) = running_apps.first() {
            generate_app_text(app, vscode_file_info.as_ref())
        } else {
            "idle".to_string()
        };

        let output = OutputData {
            text: output_text,
        };

        // Update shared data
        {
            let mut data = shared_data.write().await;
            *data = output;
        }

        // Wait before next update
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// Shit gets real
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

    // Start background task to update presence data
    tokio::spawn(async move {
        update_presence_data(data_for_task).await;
    });

    // Create and start web server
    let app = web_server::create_server(shared_data).await;
    
    println!("Represence server starting on http://0.0.0.0:3001");
    println!("API endpoint: http://0.0.0.0:3001/api/presence");
    println!("Health check: http://0.0.0.0:3001/health");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}