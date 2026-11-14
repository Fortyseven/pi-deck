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
            rpc::stop_session,
            sessions::list_sessions,
            sessions::list_children,
            sessions::read_session_messages,
            sessions::delete_session,
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
            sessions::pi_mcp_remove
        ])
        .run(tauri::generate_context!())
        .expect("failed to start tauri");
}
