pub mod app_settings;
mod commands;
mod desktop;

/// 启动 Dasktop 的 Tauri 运行时，只注册当前版本真实使用的命令和插件。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::apply_native_desktop_icon_visibility,
            commands::get_desktop_items_by_paths,
            commands::get_desktop_snapshot,
            commands::is_primary_mouse_button_pressed,
            commands::open_desktop_item,
            commands::show_native_item_context_menu
        ])
        .run(tauri::generate_context!())
        .expect("failed to run dasktop");
}
