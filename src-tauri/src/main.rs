// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 2.118.0: the phone build is the library alone (phone.rs) - no program
    // of its own to start.
    #[cfg(not(target_arch = "wasm32"))]
    tiqr_manager_lib::run();
}
