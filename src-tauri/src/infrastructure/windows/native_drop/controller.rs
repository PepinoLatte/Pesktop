//! OLE DropTarget 注册控制器，保证注册和注销都发生在窗口线程。

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

use super::drop_target::NativeDropTarget;
use super::emitter::NativeDropEmitter;
use super::window::enumerate_child_windows;
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Ole::{IDropTarget, OleInitialize, RegisterDragDrop, RevokeDragDrop};

thread_local! {
    /// OLE DropTarget 必须在窗口线程注册和释放，因此控制器保存在 UI 线程本地注册表中。
    static NATIVE_DROP_CONTROLLERS: RefCell<HashMap<String, NativeDropController>> =
        RefCell::new(HashMap::new());
}

/// DropTarget 注册结果保存每个 WebView 子 HWND 的 COM 对象，避免被 Rust 提前释放。
struct NativeDropController {
    targets: Vec<RegisteredDropTarget>,
}

/// 单个 HWND 的注册结果；`target` 字段必须随控制器存活以保持 COM 对象有效。
struct RegisteredDropTarget {
    hwnd: isize,
    target: IDropTarget,
}

impl Drop for NativeDropController {
    fn drop(&mut self) {
        for target in &self.targets {
            let _ = unsafe { RevokeDragDrop(HWND(target.hwnd as *mut c_void)) };
            let _ = &target.target;
        }
    }
}

/// 为指定 Box 窗口注册原生拖放目标；重复注册会替换旧目标。
pub fn register_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
    let app_handle = app.clone();
    let window_label = window_label.to_string();
    app.run_on_main_thread(move || {
        if let Err(error) = register_box_native_drop_on_main_thread(&app_handle, &window_label) {
            eprintln!("failed to register native drop target for {window_label}: {error}");
        }
    })
    .map_err(|error| format!("无法调度原生拖放注册：{error}"))
}

/// 前端卸载和窗口销毁都会调用注销；未注册时保持幂等。
pub fn unregister_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
    let window_label = window_label.to_string();
    app.run_on_main_thread(move || {
        NATIVE_DROP_CONTROLLERS.with(|controllers| {
            controllers.borrow_mut().remove(&window_label);
        });
    })
    .map_err(|error| format!("无法调度原生拖放注销：{error}"))
}

fn register_box_native_drop_on_main_thread(
    app: &AppHandle,
    window_label: &str,
) -> Result<(), String> {
    let webview_window = app
        .get_webview_window(window_label)
        .ok_or_else(|| "Box 窗口尚未创建，无法注册原生拖放".to_string())?;
    let hwnd = webview_window
        .hwnd()
        .map_err(|error| format!("无法获取 Box 窗口句柄：{error}"))?;
    let controller = NativeDropController::new(hwnd, app.clone(), window_label.to_string())?;
    if controller.targets.is_empty() {
        return Err("未找到可注册拖放的 WebView 子窗口".to_string());
    }

    NATIVE_DROP_CONTROLLERS.with(|controllers| {
        controllers
            .borrow_mut()
            .insert(window_label.to_string(), controller);
    });

    Ok(())
}

impl NativeDropController {
    fn new(hwnd: HWND, app: AppHandle, window_label: String) -> Result<Self, String> {
        let _ = unsafe { OleInitialize(None) };
        let mut child_windows = vec![hwnd];
        enumerate_child_windows(hwnd, &mut child_windows);

        let mut targets = Vec::new();
        let emitter = Arc::new(NativeDropEmitter::new(app, window_label));
        for child_hwnd in child_windows {
            let target: IDropTarget =
                NativeDropTarget::new(child_hwnd, Arc::clone(&emitter)).into();
            let _ = unsafe { RevokeDragDrop(child_hwnd) };

            if unsafe { RegisterDragDrop(child_hwnd, &target) }.is_ok() {
                targets.push(RegisteredDropTarget {
                    hwnd: child_hwnd.0 as isize,
                    target,
                });
            }
        }

        Ok(Self { targets })
    }
}
