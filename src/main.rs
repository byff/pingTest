// Windows: hide console window in release builds
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

fn main() {
    // Get log directory
    let log_dir: std::path::PathBuf = std::env::current_exe()
        .ok()
        .map(|p| p.parent().unwrap_or(&p).to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // Initialize logging
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).format_timestamp_millis()
     .try_init();

    // Write startup info to log file
    let start_info = format!(
        "[{}] PingTest starting...\n  Version: {}\n  OS: {}\n  Args: {:?}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::args().collect::<Vec<_>>()
    );

    let log_file = log_dir.join("pingtest.log");
    let _ = std::fs::write(&log_file, &start_info);
    eprintln!("{}", start_info);

    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--cli" | "-c" => {
                eprintln!("[DEBUG] CLI mode requested");
                run_cli_mode(&args[2..], &log_dir);
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
    }

    // Try GUI mode
    run_gui_mode(&log_dir);
}

fn run_gui_mode(log_dir: &std::path::Path) {
    eprintln!("[DEBUG] Initializing GUI mode...");
    eprintln!("[DEBUG] Creating NativeOptions...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("PingTest | 批量Ping测试工具")
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eprintln!("[DEBUG] NativeOptions created");
    eprintln!("[DEBUG] Calling eframe::run_native (glow/OpenGL renderer)");
    eprintln!("[DEBUG] Platform: {}", std::env::consts::OS);

    let result = eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| {
            eprintln!("[DEBUG] Creating PingTestApp...");
            Ok(Box::new(gui::app::PingTestApp::new(cc)))
        }),
    );

    match result {
        Ok(_) => {
            let msg = format!("[{}] PingTest exited normally\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            eprintln!("{}", msg);
            let _ = std::fs::write(log_dir.join("pingtest.log"), &msg);
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            
            let detailed_error = format!(
                "[{}] GUI ERROR:\n\
                 ====================\n\
                 Error: {}\n\
                 \n\
                 Error Analysis:\n\
                 - NoSuitableAdapterFound: {}\n\
                 - PainterError/OpenGL: {}\n\
                 - wgpu related: {}\n\
                 - glow related: {}\n\
                 \n\
                 Possible Causes:\n\
                 1. No graphics adapter (Windows Server/headless)\n\
                 2. Graphics driver not installed\n\
                 3. GPU too old for DirectX 12 / OpenGL 2.0\n\
                 4. VM without 3D acceleration\n\
                 \n\
                 Solutions:\n\
                 1. Use CLI mode: {} --cli\n\
                 2. Install graphics drivers\n\
                 3. Enable 3D in VM settings\n\
                 4. Install virtual display driver\n",
                timestamp,
                err_str,
                err_str.contains("NoSuitableAdapterFound"),
                err_str.contains("PainterError") || err_str.contains("OpenGL"),
                err_str.contains("wgpu"),
                err_str.contains("glow"),
                std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()
            );

            eprintln!("{}", detailed_error);

            // Write to log files
            let log_path = log_dir.join("pingtest_error.log");
            let panic_path = log_dir.join("pingtest_panic.log");
            let _ = std::fs::write(&log_path, &detailed_error);
            let _ = std::fs::write(&panic_path, &detailed_error);

            eprintln!("\n========== ERROR ==========");
            eprintln!("GUI failed! Check log: {}", log_path.display());
            eprintln!("Run with --cli for non-GUI mode");
        }
    }
}

fn run_cli_mode(targets: &[String], log_dir: &std::path::Path) {
    eprintln!("[DEBUG] Starting CLI mode...");
    println!("===========================================");
    println!("  PingTest CLI Mode");
    println!("===========================================\n");

    if targets.is_empty() {
        println!("Usage: pingTest --cli <IP/CIDR...>");
        return;
    }

    println!("Targets: {:?}\n", targets);
    println!("CLI ping not implemented yet.");
}

fn print_help() {
    println!(r#"PingTest v{}

Usage:
  pingTest              GUI mode
  pingTest --cli <...>  CLI mode
  pingTest --help       This help
"#, env!("CARGO_PKG_VERSION"));
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => {
            eprintln!("[DEBUG] Icon loaded");
            icon
        }
        Err(e) => {
            eprintln!("[DEBUG] Icon error: {:?}", e);
            egui::IconData::default()
        }
    }
}
