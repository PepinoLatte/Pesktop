mod commands;
pub mod domain;
mod infrastructure;
mod services;

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
            infrastructure::tauri::tray::setup_app_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::box_folder::choose_collection_root_folder,
            commands::box_folder::create_box_folder,
            commands::box_folder::delete_box_folder,
            commands::box_folder::handle_box_dragged_paths_to_desktop,
            commands::box_folder::handle_box_dropped_paths,
            commands::box_folder::migrate_box_folder,
            commands::box_folder::open_box_folder,
            commands::desktop_item::delete_desktop_items,
            commands::desktop_item::get_desktop_snapshot,
            commands::desktop_item::list_box_folder_items,
            commands::desktop_item::open_desktop_item,
            commands::desktop_item::rename_desktop_item,
            commands::desktop_item::show_native_item_context_menu,
            commands::native_drop::register_box_native_drop_target,
            commands::native_drop::unregister_box_native_drop_target,
            commands::app::is_autostart_enabled,
            commands::app::is_primary_mouse_button_pressed,
            commands::app::set_autostart_enabled,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build dasktop");

    app.run(|_app_handle, _event| {});
}
