// Windows: hide console window in release builds (GUI mode only)
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use std::path::PathBuf;

fn main() {
    // Initialize logging
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).format_timestamp_millis()
     .try_init();

    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--cli" | "-c" => {
                log::info!("Running in CLI mode");
                run_cli_mode(&args[2..]);
                return;
            }
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}\n", args[1]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Try GUI mode
    run_gui_mode();
}

fn run_gui_mode() {
    log::info!("PingTest starting (GUI mode)...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("PingTest | 批量Ping测试工具")
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    log::info!("Calling eframe::run_native (glow+wgpu enabled)...");

    match eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| {
            log::info!("Creating PingTestApp...");
            Ok(Box::new(gui::app::PingTestApp::new(cc)))
        }),
    ) {
        Ok(_) => {
            log::info!("PingTest exited normally");
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            log::error!("eframe error: {}", err_str);

            // Write error to file
            if let Ok(exe) = std::env::current_exe() {
                let log_path = exe.parent().unwrap_or(&exe).join("pingtest_error.log");
                let _ = std::fs::write(&log_path, format!("{}\n", err_str));
            }

            // Check specific error types and provide helpful messages
            if err_str.contains("NoSuitableAdapterFound") {
                eprintln!("\n错误: 找不到合适的图形适配器");
                eprintln!("\n可能的原因:");
                eprintln!("  1. 你的显卡太旧，不支持 DirectX 12 或 OpenGL 2.0");
                eprintln!("  2. Windows Server 没有安装图形驱动");
                eprintln!("  3. 虚拟机没有启用 3D 加速");
                eprintln!("\n解决方案:");
                eprintln!("  - 使用 CLI 模式: {} --cli", 
                    std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
                eprintln!("  - Windows Server: 安装虚拟显示驱动或启用桌面体验");
                eprintln!("  - 虚拟机: 启用 3D 加速");
            } else if err_str.contains("PainterError") || err_str.contains("OpenGL") {
                eprintln!("\n错误: OpenGL 初始化失败");
                eprintln!("\n你的显卡可能不支持 OpenGL 2.0+");
                eprintln!("\n解决方案:");
                eprintln!("  - 使用 CLI 模式: {} --cli", 
                    std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
            } else {
                eprintln!("\nGUI 启动失败: {}", err_str);
                eprintln!("\n可以使用 CLI 模式运行: {} --cli", 
                    std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
            }
        }
    }
}

fn run_cli_mode(targets: &[String]) {
    println!("===========================================");
    println!("  PingTest CLI 模式 (无 GUI)");
    println!("===========================================\n");

    if targets.is_empty() {
        eprintln!("用法: pingTest --cli <IP地址或网段...>");
        eprintln!("示例: pingTest --cli 192.168.1.1 10.0.0.0/24");
        eprintln!("\n或者从文件导入:");
        eprintln!("  pingTest --cli $(cat ips.txt)");
        return;
    }

    println!("将 ping 以下目标: {:?}\n", targets);

    // TODO: Implement actual CLI ping functionality
    println!("CLI ping 功能开发中...");
    println!("\n提示: GUI 版本需要图形支持，请在有显卡的机器上运行。");
}

fn print_help() {
    println!(r#"
PingTest - 批量Ping测试工具

用法:
  pingTest              启动 GUI 版本
  pingTest --cli <目标> 启动 CLI 版本 (无需图形)
  pingTest --help       显示此帮助

选项:
  -h, --help           显示帮助
  -c, --cli            CLI 模式，无需图形界面

示例:
  pingTest --cli 192.168.1.1
  pingTest --cli 10.0.0.0/24 192.168.0.1
"#);
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => icon,
        Err(_) => egui::IconData::default(),
    }
}
