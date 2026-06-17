pub mod app_settings;
mod commands;
mod desktop;

/// 启动 Dasktop 的 Tauri 运行时，只注册当前版本真实使用的命令和插件。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|_app| {
            if let Err(error) = desktop::set_native_desktop_icons_hidden(false) {
                eprintln!("failed to show native desktop icons on startup: {error}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_desktop_items_by_paths,
            commands::get_desktop_snapshot,
            commands::is_primary_mouse_button_pressed,
            commands::open_desktop_item,
            commands::set_native_desktop_icons_hidden,
            commands::show_native_item_context_menu,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build dasktop");

    app.run(|_app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            if let Err(error) = desktop::set_native_desktop_icons_hidden(false) {
                eprintln!("failed to show native desktop icons on exit: {error}");
            }
        }
    });
}
