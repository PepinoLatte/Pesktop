pub mod app_settings;
mod app_tray;
mod commands;
mod desktop;

/// 启动 Dasktop 的 Tauri 运行时，只注册当前版本真实使用的命令和插件
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|app| {
            app_tray::setup_app_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::choose_collection_root_folder,
            commands::create_box_folder,
            commands::delete_desktop_items,
            commands::delete_box_folder,
            commands::get_desktop_snapshot,
            commands::handle_box_dragged_paths_to_desktop,
            commands::handle_box_dropped_paths,
            commands::is_autostart_enabled,
            commands::is_primary_mouse_button_pressed,
            commands::list_box_folder_items,
            commands::migrate_box_folder,
            commands::open_desktop_item,
            commands::open_box_folder,
            commands::register_box_native_drop_target,
            commands::rename_desktop_item,
            commands::set_autostart_enabled,
            commands::show_native_item_context_menu,
            commands::unregister_box_native_drop_target,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build dasktop");

    app.run(|_app_handle, _event| {});
}
