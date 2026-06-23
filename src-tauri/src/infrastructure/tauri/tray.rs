//! 系统托盘入口负责应用级命令分发，不直接读写 Box 数据。

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{CheckMenuItem, MenuBuilder, MenuEvent};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, Window, WindowEvent, Wry};
use tauri_plugin_autostart::ManagerExt;

const TRAY_ID: &str = "dasktop-tray";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_CREATE_BOX: &str = "create-box";
const MENU_ID_AUTOSTART: &str = "autostart";
const MENU_ID_QUIT: &str = "quit";

/// 托盘请求创建 Box 的前端事件；main WebView 作为隐藏控制器复用现有 Store 创建流程。
pub const TRAY_CREATE_BOX_EVENT: &str = "dasktop://tray-create-box";
/// 自启状态变化事件用于同步隐藏设置窗里的开关状态。
pub const AUTOSTART_CHANGED_EVENT: &str = "dasktop://autostart-changed";

/// 托盘菜单中需要跨入口同步的可变控件状态。
pub struct AppTrayState {
    autostart_item: CheckMenuItem<Wry>,
    /// 托盘退出会真正销毁所有窗口，此标记用于区分系统关闭设置窗和应用退出。
    is_graceful_exit_requested: AtomicBool,
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
    let menu = MenuBuilder::new(app)
        .text(MENU_ID_SETTINGS, "设置")
        .text(MENU_ID_CREATE_BOX, "新增 Box")
        .separator()
        .item(&autostart_item)
        .separator()
        .text(MENU_ID_QUIT, "关闭")
        .build()?;
    let autostart_item_for_menu = autostart_item.clone();
    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Dasktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            handle_tray_menu_event(app_handle, &event, &autostart_item_for_menu);
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
        is_graceful_exit_requested: AtomicBool::new(false),
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

fn handle_tray_menu_event(
    app_handle: &AppHandle,
    event: &MenuEvent,
    autostart_item: &CheckMenuItem<Wry>,
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
        MENU_ID_QUIT => request_graceful_exit(app_handle),
        _ => {}
    }
}

/// 托盘“关闭”是真正退出应用，需要让所有 WebViewWindow 先走销毁链路。
///
/// Windows WebView2/Chromium 在进程退出时会注销 `Chrome_WidgetWin_0` 等内部窗口类。
/// 如果直接调用 `AppHandle::exit`，隐藏设置窗、Box 窗口和预载菜单窗可能尚未收到
/// `Destroyed` 事件，底层清理就会和窗口销毁交错，从而打印 class unregister 失败日志。
/// 这里逐个销毁现有 WebViewWindow，并用退出标记跳过设置窗的隐藏拦截；系统桌面图标
/// 由 `RunEvent::ExitRequested` 按接管记录恢复到 Dasktop 启动前状态。
pub(crate) fn request_graceful_exit(app_handle: &AppHandle) {
    if let Some(tray_state) = app_handle.try_state::<AppTrayState>() {
        tray_state
            .is_graceful_exit_requested
            .store(true, Ordering::SeqCst);
    }

    let webview_windows = app_handle.webview_windows();
    if webview_windows.is_empty() {
        app_handle.exit(0);
        return;
    }

    for (label, webview_window) in webview_windows {
        if let Err(error) = webview_window.destroy() {
            eprintln!("failed to destroy webview window {label} before exit: {error}");
        }
    }
}

/// 任务栏或系统菜单关闭设置窗时只隐藏主窗口，保留隐藏控制器和 Store 事件入口供托盘再次唤起。
pub(crate) fn handle_window_close_requested(window: &Window<Wry>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != "main" {
        return;
    }
    let Some(tray_state) = window.app_handle().try_state::<AppTrayState>() else {
        return;
    };
    if tray_state.is_graceful_exit_requested.load(Ordering::SeqCst) {
        return;
    }

    api.prevent_close();
    if let Err(error) = window.hide() {
        eprintln!("failed to hide settings window after close request: {error}");
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

pub(crate) fn show_settings_window(app_handle: &AppHandle) {
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
