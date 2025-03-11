// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cursor_agent_lib::*;
use tauri::Builder;
use tauri::generate_handler;
use tauri::generate_context;
use crate::utils::privileges::{check_admin_privileges, request_admin_privileges};
use crate::utils::process::ProcessManager;
use std::env;

fn main() {
    // Windows 平台下检查管理员权限
    #[cfg(target_os = "windows")]
    {
        if let Ok(false) = check_admin_privileges() {
            let exe_path = env::current_exe()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            if let Ok(true) = request_admin_privileges(&exe_path) {
                std::process::exit(0);
            }
        }
    }

    let process_manager = ProcessManager::new();
    if process_manager.is_other_cursor_pool_running() {
        if let Err(e) = process_manager.kill_other_cursor_pool_processes() {
            eprintln!("终止其他 Cursor Agent 实例失败: {}", e);
            std::process::exit(1);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            tray::setup_system_tray(app)?;
            Ok(())
        })
        .invoke_handler(generate_handler![
            api::login,
            api::get_user_info,
            api::activate,
            api::change_password,
            api::get_account,
            api::get_usage,
            api::get_user_info_cursor,
            api::check_user,
            api::send_code,
            api::get_version,
            api::get_public_info,
            api::reset_password,
            api::get_disclaimer,
            api::register,
            api::get_tenant_id,
            reset_machine_id,
            switch_account,
            get_machine_ids,
            cursor_reset::commands::check_cursor_running,
            cursor_reset::commands::check_admin_privileges,
            cursor_reset::commands::disable_cursor_update,
            cursor_reset::commands::restore_cursor_update,
            cursor_reset::commands::check_update_disabled,
            cursor_reset::commands::is_hook,
            cursor_reset::commands::hook_main_js,
            cursor_reset::commands::restore_hook,
            cursor_reset::commands::check_is_windows,
            cursor_reset::commands::close_cursor,
            cursor_reset::commands::launch_cursor,
        ])
        .manage(api::ApiClient::default())
        .run(generate_context!())
        .expect("error while running tauri application");
}