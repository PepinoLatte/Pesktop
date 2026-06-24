//! 托盘菜单定义和菜单事件分发。

use crate::domain::app::contract::{SETTINGS_WINDOW_LABEL, TRAY_CREATE_BOX_EVENT};
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuEvent};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Wry};

use super::autostart;
use super::window;

/// Tauri 托盘 ID，集中命名便于后续定位托盘实例。
pub(super) const TRAY_ID: &str = "dasktop-tray";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_CREATE_BOX: &str = "create-box";
const MENU_ID_AUTOSTART: &str = "autostart";
const MENU_ID_QUIT: &str = "quit";

/// 创建托盘菜单和需要跨入口同步的开机自启菜单项。
pub(super) fn build_tray_menu(
    app: &App,
    autostart_enabled: bool,
) -> tauri::Result<(Menu<Wry>, CheckMenuItem<Wry>)> {
    let autostart_item = CheckMenuItem::with_id(
        app,
        MENU_ID_AUTOSTART,
        "开机自启",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let tray_menu = MenuBuilder::new(app)
        .text(MENU_ID_SETTINGS, "设置")
        .text(MENU_ID_CREATE_BOX, "新增 Box")
        .separator()
        .item(&autostart_item)
        .separator()
        .text(MENU_ID_QUIT, "关闭")
        .build()?;

    Ok((tray_menu, autostart_item))
}

/// 根据菜单 ID 分发托盘动作；业务数据创建仍交给前端 Store 处理。
pub(super) fn handle_tray_menu_event(
    app_handle: &AppHandle,
    event: &MenuEvent,
    autostart_item: &CheckMenuItem<Wry>,
) {
    match event.id().as_ref() {
        MENU_ID_SETTINGS => window::show_settings_window(app_handle),
        MENU_ID_CREATE_BOX => request_create_box(app_handle),
        MENU_ID_AUTOSTART => {
            let next_enabled = autostart_item.is_checked().unwrap_or(false);
            if let Err(error) = autostart::set_autostart_enabled(app_handle, next_enabled) {
                eprintln!("{error}");
                if let Ok(current_enabled) = autostart::resolve_autostart_enabled(app_handle) {
                    autostart::sync_autostart_state(app_handle, current_enabled);
                }
            }
        }
        MENU_ID_QUIT => window::request_graceful_exit(app_handle),
        _ => {}
    }
}

/// 托盘左键释放打开设置窗，按释放触发可以避免按下时与系统菜单状态冲突。
pub(super) fn is_left_click_release(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

fn request_create_box(app_handle: &AppHandle) {
    if let Err(error) = app_handle.emit_to(SETTINGS_WINDOW_LABEL, TRAY_CREATE_BOX_EVENT, ()) {
        eprintln!("failed to request tray box creation: {error}");
    }
}
