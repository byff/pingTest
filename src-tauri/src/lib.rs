mod config;
mod excel;
mod ping;
mod utils;

use std::sync::Arc;
use parking_lot::RwLock;
use tauri::{Manager, State};

// Re-export types for use in commands
pub use config::{AppConfig, PingConfig, DisplayConfig, ExportConfig};
pub use ping::{PingEngine, PingTarget, PingStats};
pub use utils::{parse_targets, extract_and_clean_ips, count_cidr_ips};

// Application state shared across commands
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub ping_engine: Arc<RwLock<Option<PingEngine>>>,
    pub targets: Arc<RwLock<Vec<PingTarget>>>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfig::load())),
            ping_engine: Arc::new(RwLock::new(None)),
            targets: Arc::new(RwLock::new(Vec::new())),
            runtime: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime"),
            ),
        }
    }
}

// ============================================================================
// Tauri Commands - Configuration
// ============================================================================

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    Ok(state.config.read().clone())
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config.save();
    *state.config.write() = config;
    Ok(())
}

#[tauri::command]
fn update_ping_config(state: State<AppState>, ping_config: PingConfig) -> Result<(), String> {
    state.config.write().ping = ping_config;
    Ok(())
}

// ============================================================================
// Tauri Commands - Target Management
// ============================================================================

#[tauri::command]
fn parse_input_targets(
    state: State<AppState>,
    input: String,
    strip_first_last: bool,
) -> Result<Vec<TargetInfo>, String> {
    let config = state.config.read();
    let (targets, skipped) = parse_targets(&input, strip_first_last);
    
    Ok(targets.into_iter()
        .enumerate()
        .map(|(i, (hostname, ip))| TargetInfo {
            index: i,
            hostname,
            ip: ip.to_string(),
            success_count: 0,
            fail_count: 0,
            total_sent: 0,
            last_rtt_ms: None,
            max_rtt_ms: 0,
            min_rtt_ms: 0,
            avg_rtt_ms: 0,
            fail_rate: 0.0,
            is_alive: false,
            skipped,
        })
        .collect())
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TargetInfo {
    pub index: usize,
    pub hostname: String,
    pub ip: String,
    pub success_count: u64,
    pub fail_count: u64,
    pub total_sent: u64,
    pub last_rtt_ms: Option<f64>,
    pub max_rtt_ms: f64,
    pub min_rtt_ms: f64,
    pub avg_rtt_ms: f64,
    pub fail_rate: f64,
    pub is_alive: bool,
    pub skipped: usize,
}

impl From<&PingTarget> for TargetInfo {
    fn from(target: &PingTarget) -> Self {
        let stats = target.stats.read();
        TargetInfo {
            index: target.index,
            hostname: target.hostname.clone(),
            ip: target.ip.to_string(),
            success_count: stats.success_count,
            fail_count: stats.fail_count,
            total_sent: stats.total_sent,
            last_rtt_ms: stats.last_rtt_us.map(|u| u as f64 / 1000.0),
            max_rtt_ms: stats.max_rtt_us as f64 / 1000.0,
            min_rtt_ms: stats.min_rtt_us as f64 / 1000.0,
            avg_rtt_ms: stats.avg_rtt_us() as f64 / 1000.0,
            fail_rate: stats.fail_rate(),
            is_alive: stats.is_alive,
            skipped: 0,
        }
    }
}

#[tauri::command]
fn set_targets(state: State<AppState>, targets_input: Vec<TargetInput>) -> Result<(), String> {
    let config = state.config.read().clone();
    let ping_config = &config.ping;
    
    let mut ping_targets: Vec<PingTarget> = targets_input
        .into_iter()
        .enumerate()
        .map(|(i, input)| {
            let ip: std::net::IpAddr = input.ip.parse()
                .map_err(|_| format!("Invalid IP address: {}", input.ip))?;
            Ok(PingTarget {
                index: i,
                hostname: input.hostname,
                ip,
                stats: Arc::new(RwLock::new(PingStats::default())),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    
    let mut engine = PingEngine::new(
        ping_config.timeout_ms,
        ping_config.interval_ms,
        ping_config.packet_size,
        ping_config.max_concurrent,
    );
    engine.set_targets(ping_targets);
    
    *state.ping_engine.write() = Some(engine);
    Ok(())
}

#[derive(serde::Deserialize, Clone)]
pub struct TargetInput {
    pub ip: String,
    pub hostname: String,
}

// ============================================================================
// Tauri Commands - Ping Control
// ============================================================================

#[tauri::command]
fn start_ping(state: State<AppState>) -> Result<(), String> {
    let engine_guard = state.ping_engine.read();
    if let Some(ref engine) = *engine_guard {
        if engine.is_running() {
            return Err("Ping is already running".to_string());
        }
        engine.start(&state.runtime.handle().clone());
        Ok(())
    } else {
        Err("No targets configured. Call set_targets first.".to_string())
    }
}

#[tauri::command]
fn stop_ping(state: State<AppState>) -> Result<(), String> {
    let engine_guard = state.ping_engine.read();
    if let Some(ref engine) = *engine_guard {
        engine.stop();
        Ok(())
    } else {
        Err("No ping engine initialized".to_string())
    }
}

#[tauri::command]
fn get_ping_stats(state: State<AppState>) -> Result<Vec<TargetInfo>, String> {
    let engine_guard = state.ping_engine.read();
    if let Some(ref engine) = *engine_guard {
        let targets = engine.targets();
        Ok(targets.iter().map(TargetInfo::from).collect())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn is_ping_running(state: State<AppState>) -> Result<bool, String> {
    let engine_guard = state.ping_engine.read();
    Ok(if let Some(ref engine) = *engine_guard {
        engine.is_running()
    } else {
        false
    })
}

// ============================================================================
// Tauri Commands - Excel Import/Export
// ============================================================================

#[tauri::command]
fn read_excel_file(path: String) -> Result<ExcelData, String> {
    let path = std::path::Path::new(&path);
    let (headers, rows) = excel::read_excel(path)?;
    Ok(ExcelData { headers, rows })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExcelData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[tauri::command]
fn export_to_excel(
    state: State<AppState>,
    path: String,
) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    let engine_guard = state.ping_engine.read();
    let config_guard = state.config.read();
    
    if let Some(ref engine) = *engine_guard {
        let targets = engine.targets();
        excel::export_results(path, targets, &config_guard.export)?;
        Ok(())
    } else {
        Err("No ping results to export".to_string())
    }
}

// ============================================================================
// Tauri Commands - Utility
// ============================================================================

#[tauri::command]
fn clean_ip_input(input: String) -> Result<String, String> {
    Ok(extract_and_clean_ips(&input))
}

#[tauri::command]
fn count_targets(input: String) -> Result<usize, String> {
    Ok(count_cidr_ips(&input))
}

#[tauri::command]
fn get_stats_summary(state: State<AppState>) -> Result<StatsSummary, String> {
    let engine_guard = state.ping_engine.read();
    if let Some(ref engine) = *engine_guard {
        let targets = engine.targets();
        let total = targets.len();
        let alive = targets.iter().filter(|t| t.stats.read().is_alive).count();
        let total_success: u64 = targets.iter().map(|t| t.stats.read().success_count).sum();
        let total_fail: u64 = targets.iter().map(|t| t.stats.read().fail_count).sum();
        
        Ok(StatsSummary {
            total_targets: total,
            alive_count: alive,
            dead_count: total - alive,
            total_success,
            total_fail,
            running: engine.is_running(),
        })
    } else {
        Ok(StatsSummary {
            total_targets: 0,
            alive_count: 0,
            dead_count: 0,
            total_success: 0,
            total_fail: 0,
            running: false,
        })
    }
}

#[derive(serde::Serialize)]
pub struct StatsSummary {
    pub total_targets: usize,
    pub alive_count: usize,
    pub dead_count: usize,
    pub total_success: u64,
    pub total_fail: u64,
    pub running: bool,
}

// ============================================================================
// App Builder
// ============================================================================

pub fn run() {
    let app_state = AppState::new();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Config commands
            get_config,
            save_config,
            update_ping_config,
            // Target commands
            parse_input_targets,
            set_targets,
            // Ping control
            start_ping,
            stop_ping,
            get_ping_stats,
            is_ping_running,
            // Excel commands
            read_excel_file,
            export_to_excel,
            // Utility commands
            clean_ip_input,
            count_targets,
            get_stats_summary,
        ])
        .setup(|app| {
            log::info!("PingTest Tauri app starting...");
            
            // Configure window
            if let Some(window) = app.get_webview_window("main") {
                let config = AppConfig::load();
                let _ = window.set_title("PingTest | 批量Ping测试工具");
                #[cfg(desktop)]
                {
                    use tauri::LogicalSize;
                    let _ = window.set_size(LogicalSize::new(config.window_width as f64, config.window_height as f64));
                }
            }
            
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Save window size on close
                if let Ok(size) = window.inner_size() {
                    let mut config = AppConfig::load();
                    config.window_width = size.width as f32;
                    config.window_height = size.height as f32;
                    config.save();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
