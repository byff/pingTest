// Windows: hide console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use gui::app::PingTestApp;

fn write_panic(panic_info: &std::panic::PanicInfo) {
    let msg = format!("PANIC: {}\n", panic_info);
    if let Some(exe) = std::env::current_exe().ok() {
        let log_path = exe.with_file_name("pingtest_panic.log");
        let _ = std::fs::write(&log_path, &msg);
    }
    let _ = std::eprintln!("{}", &msg);
}

fn main() {
    // Set up panic hook FIRST
    std::panic::set_hook(Box::new(|pi| write_panic(pi)));

    log::info!("PingTest starting...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("PingTest | 批量Ping测试工具")
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    log::info!("Calling eframe::run_native...");

    if let Err(e) = eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| {
            log::info!("Creating PingTestApp...");
            Ok(Box::new(PingTestApp::new(cc)))
        }),
    ) {
        log::error!("eframe error: {:?}", e);
        // Also write to file
        if let Some(exe) = std::env::current_exe().ok() {
            let msg = format!("eframe error: {:?}\n", e);
            let _ = std::fs::write(exe.with_file_name("pingtest_error.log"), &msg);
        }
    }

    log::info!("PingTest exited");
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => icon,
        Err(_) => egui::IconData::default(),
    }
}
