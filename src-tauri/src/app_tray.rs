//! 系统托盘入口负责应用级命令分发，不直接读写 Box 数据。

use tauri::menu::{CheckMenuItem, MenuBuilder, MenuEvent};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

const TRAY_ID: &str = "dasktop-tray";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_CREATE_BOX: &str = "create-box";
const MENU_ID_AUTOSTART: &str = "autostart";
const MENU_ID_NATIVE_DESKTOP_ICONS_HIDDEN: &str = "native-desktop-icons-hidden";
const MENU_ID_QUIT: &str = "quit";

/// 托盘请求创建 Box 的前端事件；main WebView 作为隐藏控制器复用现有 Store 创建流程。
pub const TRAY_CREATE_BOX_EVENT: &str = "dasktop://tray-create-box";
/// 自启状态变化事件用于同步隐藏设置窗里的开关状态。
pub const AUTOSTART_CHANGED_EVENT: &str = "dasktop://autostart-changed";
/// 托盘请求切换 Explorer 原生桌面图标层，由前端 Store 继续负责持久化和系统命令调用。
pub const TRAY_TOGGLE_NATIVE_DESKTOP_ICONS_HIDDEN_EVENT: &str =
    "dasktop://tray-toggle-native-desktop-icons-hidden";

/// 托盘菜单中需要跨入口同步的可变控件状态。
pub struct AppTrayState {
    autostart_item: CheckMenuItem<Wry>,
    native_desktop_icons_hidden_item: CheckMenuItem<Wry>,
}

/// 初始化系统托盘图标和右键菜单；菜单只分发动作，具体 Box 生命周期仍由前端统一处理。
pub fn setup_app_tray(app: &App) -> tauri::Result<()> {
    let autostart_enabled = resolve_autostart_enabled(app.handle()).unwrap_or(false);
    let autostart_item = CheckMenuItem::with_id(
        app,
        MENU_ID_AUTOSTART,
        "开机自启",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let native_desktop_icons_hidden_item = CheckMenuItem::with_id(
        app,
        MENU_ID_NATIVE_DESKTOP_ICONS_HIDDEN,
        "隐藏系统桌面图标",
        true,
        false,
        None::<&str>,
    )?;
    let menu = MenuBuilder::new(app)
        .text(MENU_ID_SETTINGS, "设置")
        .text(MENU_ID_CREATE_BOX, "新增 Box")
        .separator()
        .item(&autostart_item)
        .item(&native_desktop_icons_hidden_item)
        .separator()
        .text(MENU_ID_QUIT, "关闭")
        .build()?;
    let autostart_item_for_menu = autostart_item.clone();
    let native_desktop_icons_hidden_item_for_menu = native_desktop_icons_hidden_item.clone();
    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Dasktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            handle_tray_menu_event(
                app_handle,
                &event,
                &autostart_item_for_menu,
                &native_desktop_icons_hidden_item_for_menu,
            );
        })
        .on_tray_icon_event(|tray, event| {
            if is_left_click_release(&event) {
                show_settings_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app)?;
    app.manage(AppTrayState {
        autostart_item,
        native_desktop_icons_hidden_item,
    });
    Ok(())
}

/// 读取插件中的系统自启状态，供设置页和托盘初始化共享。
pub fn resolve_autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("无法读取开机自启状态：{error}"))
}

/// 写入系统自启状态并同步所有 UI 入口，避免设置页和托盘出现互相矛盾的勾选状态。
pub fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch
            .enable()
            .map_err(|error| format!("无法开启开机自启：{error}"))?;
    } else {
        autolaunch
            .disable()
            .map_err(|error| format!("无法关闭开机自启：{error}"))?;
    }

    sync_autostart_state(app, enabled);
    Ok(enabled)
}

/// 同步托盘中“隐藏系统桌面图标”勾选状态；真实偏好仍以设置页 Store 为准。
pub fn set_native_desktop_icons_hidden_checked(
    app: &AppHandle,
    hidden: bool,
) -> Result<bool, String> {
    let tray_state = app
        .try_state::<AppTrayState>()
        .ok_or_else(|| "系统托盘尚未初始化，无法同步桌面图标隐藏状态".to_string())?;
    tray_state
        .native_desktop_icons_hidden_item
        .set_checked(hidden)
        .map_err(|error| format!("无法同步托盘桌面图标隐藏状态：{error}"))?;

    Ok(hidden)
}

fn handle_tray_menu_event(
    app_handle: &AppHandle,
    event: &MenuEvent,
    autostart_item: &CheckMenuItem<Wry>,
    native_desktop_icons_hidden_item: &CheckMenuItem<Wry>,
) {
    match event.id().as_ref() {
        MENU_ID_SETTINGS => show_settings_window(app_handle),
        MENU_ID_CREATE_BOX => request_create_box(app_handle),
        MENU_ID_AUTOSTART => {
            let next_enabled = autostart_item.is_checked().unwrap_or(false);
            if let Err(error) = set_autostart_enabled(app_handle, next_enabled) {
                eprintln!("{error}");
                if let Ok(current_enabled) = resolve_autostart_enabled(app_handle) {
                    sync_autostart_state(app_handle, current_enabled);
                }
            }
        }
        MENU_ID_NATIVE_DESKTOP_ICONS_HIDDEN => {
            request_toggle_native_desktop_icons_hidden(
                app_handle,
                native_desktop_icons_hidden_item
                    .is_checked()
                    .unwrap_or(false),
            );
        }
        MENU_ID_QUIT => app_handle.exit(0),
        _ => {}
    }
}

fn is_left_click_release(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

fn show_settings_window(app_handle: &AppHandle) {
    if let Some(settings_window) = app_handle.get_webview_window("main") {
        if let Err(error) = settings_window.unminimize() {
            eprintln!("failed to unminimize settings window: {error}");
        }
        if let Err(error) = settings_window.show() {
            eprintln!("failed to show settings window: {error}");
        }
        if let Err(error) = settings_window.set_focus() {
            eprintln!("failed to focus settings window: {error}");
        }
    }
}

fn request_create_box(app_handle: &AppHandle) {
    if let Err(error) = app_handle.emit_to("main", TRAY_CREATE_BOX_EVENT, ()) {
        eprintln!("failed to request tray box creation: {error}");
    }
}

fn request_toggle_native_desktop_icons_hidden(app_handle: &AppHandle, hidden: bool) {
    if let Err(error) = app_handle.emit_to(
        "main",
        TRAY_TOGGLE_NATIVE_DESKTOP_ICONS_HIDDEN_EVENT,
        hidden,
    ) {
        eprintln!("failed to request native desktop icons visibility toggle: {error}");
    }
}

fn sync_autostart_state(app: &AppHandle, enabled: bool) {
    if let Some(tray_state) = app.try_state::<AppTrayState>() {
        if let Err(error) = tray_state.autostart_item.set_checked(enabled) {
            eprintln!("failed to sync tray autostart check state: {error}");
        }
    }
    if let Err(error) = app.emit_to("main", AUTOSTART_CHANGED_EVENT, enabled) {
        eprintln!("failed to emit autostart state change: {error}");
    }
}
