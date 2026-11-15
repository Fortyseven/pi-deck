#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rpc;
mod sessions;

fn main() {
    tauri::Builder::default()
        .manage(rpc::RpcState::default())
        .invoke_handler(tauri::generate_handler![
            rpc::start_session,
            rpc::send_prompt,
            rpc::rpc_command,
            rpc::running_sessions,
            rpc::stop_session,
            sessions::list_sessions,
            sessions::list_children,
            sessions::read_session_messages,
            sessions::session_model,
            sessions::pi_models,
            sessions::delete_session,
            sessions::kill_orphans,
            sessions::hooks_status,
            sessions::hooks_install,
            sessions::hooks_remove,
            sessions::hooks_tail,
            sessions::home_dir,
            sessions::debug_flags,
            sessions::debug_note,
            sessions::pi_version,
            sessions::pi_update,
            sessions::pi_list,
            sessions::pi_mcp_list,
            sessions::pi_install,
            sessions::pi_remove,
            sessions::pi_mcp_add,
            sessions::pi_mcp_remove,
            sessions::app_version,
            sessions::open_external,
            sessions::copy_text,
            sessions::auth_status,
            sessions::auth_providers,
            sessions::pick_directory,
            sessions::pick_file,
            sessions::paste_image,
            sessions::open_terminal_login
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                use tauri::Manager;
                window.state::<rpc::RpcState>().kill_all();
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start tauri");
}
