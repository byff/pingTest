// Windows: hide console window in release builds (GUI mode only)
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

fn main() {
    pingtest_lib::run();
}
