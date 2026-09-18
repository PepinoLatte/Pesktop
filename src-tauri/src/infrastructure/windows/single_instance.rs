//! Windows 单实例控制，负责阻止重复进程并把二次启动转换成主实例内的设置窗口唤起。

#[cfg(target_os = "windows")]
use std::thread;

#[cfg(target_os = "windows")]
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE,
};
#[cfg(target_os = "windows")]
use windows_core::{w, Owned};

#[cfg(target_os = "windows")]
const INSTANCE_MUTEX_NAME: windows_core::PCWSTR = w!("Local\\Dasktop.cn.syand.dasktop.Instance");
#[cfg(target_os = "windows")]
const SHOW_SETTINGS_EVENT_NAME: windows_core::PCWSTR =
    w!("Local\\Dasktop.cn.syand.dasktop.ShowSettings");
#[cfg(target_os = "windows")]
const RELOAD_FRONTEND_EVENT_NAME: windows_core::PCWSTR =
    w!("Local\\Dasktop.cn.syand.dasktop.ReloadFrontend");

/// 单实例锁需要在 Tauri 运行期内持续持有，否则 Windows 命名互斥量会被自动释放。
#[cfg(target_os = "windows")]
pub(crate) struct SingleInstanceGuard {
    _mutex: OwnedKernelHandle,
    _show_settings_event: OwnedKernelHandle,
}

/// 非 Windows 平台暂不提供进程级单实例锁，保持调用方跨平台编译路径稳定。
#[cfg(not(target_os = "windows"))]
pub(crate) struct SingleInstanceGuard;

/// 获取当前用户会话内的 Dasktop 单实例锁；若已有主实例，则通知它显示设置窗口并退出当前进程。
#[cfg(target_os = "windows")]
pub(crate) fn acquire_or_notify_existing() -> Result<Option<SingleInstanceGuard>, String> {
    let mutex = unsafe { CreateMutexW(None, true, INSTANCE_MUTEX_NAME) }
        .map_err(|error| format!("无法创建 Dasktop 单实例锁：{error}"))?;
    let mutex_status = unsafe { GetLastError() };
    let mutex = unsafe { OwnedKernelHandle::new(mutex) };

    if mutex_status == ERROR_ALREADY_EXISTS {
        // 带 --reload 的二次启动是热更触发信号，通知主实例重载界面而不是弹设置窗
        if is_reload_invocation() {
            notify_primary_instance_to_reload_frontend()?;
        } else {
            notify_primary_instance_to_show_settings()?;
        }
        return Ok(None);
    }

    let show_settings_event = create_show_settings_event()?;
    Ok(Some(SingleInstanceGuard {
        _mutex: mutex,
        _show_settings_event: show_settings_event,
    }))
}

/// 非 Windows 平台直接允许继续启动，避免把 Windows 互斥量策略泄漏到其他平台。
#[cfg(not(target_os = "windows"))]
pub(crate) fn acquire_or_notify_existing() -> Result<Option<SingleInstanceGuard>, String> {
    Ok(Some(SingleInstanceGuard))
}

/// 主实例启动后创建命名事件监听线程，二次启动只需触发该事件即可唤起设置页。
#[cfg(target_os = "windows")]
pub(crate) fn start_show_settings_listener(app_handle: AppHandle) -> Result<(), String> {
    let event = create_show_settings_event()?;

    thread::Builder::new()
        .name("dasktop-single-instance-listener".to_string())
        .spawn(move || wait_for_show_settings_requests(event, app_handle))
        .map(|_| ())
        .map_err(|error| format!("无法启动 Dasktop 单实例监听线程：{error}"))
}

/// 主实例另起线程监听热更重载事件；`--reload` 二次启动触发后全窗口换新前端资源。
#[cfg(target_os = "windows")]
pub(crate) fn start_reload_frontend_listener(app_handle: AppHandle) -> Result<(), String> {
    let event = create_reload_frontend_event()?;

    thread::Builder::new()
        .name("dasktop-reload-frontend-listener".to_string())
        .spawn(move || wait_for_reload_frontend_requests(event, app_handle))
        .map(|_| ())
        .map_err(|error| format!("无法启动 Dasktop 热更重载监听线程：{error}"))
}

/// 非 Windows 平台不启动监听线程，接口保留给主流程统一调用。
#[cfg(not(target_os = "windows"))]
pub(crate) fn start_show_settings_listener(_app_handle: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

/// 非 Windows 平台不启动重载监听线程，接口保留给主流程统一调用。
#[cfg(not(target_os = "windows"))]
pub(crate) fn start_reload_frontend_listener(_app_handle: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn notify_primary_instance_to_show_settings() -> Result<(), String> {
    let event = create_show_settings_event()?;

    unsafe { SetEvent(event.get()) }.map_err(|error| format!("无法通知已运行的 Dasktop：{error}"))
}

/// 判定本次进程启动是否为热更触发：脚本以 `Dasktop.exe --reload` 形式二次启动。
#[cfg(target_os = "windows")]
fn is_reload_invocation() -> bool {
    std::env::args()
        .skip(1)
        .any(|argument| argument == "--reload")
}

#[cfg(target_os = "windows")]
fn notify_primary_instance_to_reload_frontend() -> Result<(), String> {
    let event = create_reload_frontend_event()?;

    unsafe { SetEvent(event.get()) }.map_err(|error| format!("无法通知主实例重载前端：{error}"))
}

#[cfg(target_os = "windows")]
fn wait_for_show_settings_requests(event: OwnedKernelHandle, app_handle: AppHandle) {
    loop {
        let wait_result = unsafe { WaitForSingleObject(event.get(), INFINITE) };
        if wait_result != WAIT_OBJECT_0 {
            break;
        }

        let app_handle_for_main_thread = app_handle.clone();
        if let Err(error) = app_handle.run_on_main_thread(move || {
            crate::infrastructure::tauri::tray::show_settings_window(&app_handle_for_main_thread);
        }) {
            eprintln!("failed to dispatch single-instance settings request: {error}");
            break;
        }
    }
}

#[cfg(target_os = "windows")]
fn wait_for_reload_frontend_requests(event: OwnedKernelHandle, app_handle: AppHandle) {
    loop {
        let wait_result = unsafe { WaitForSingleObject(event.get(), INFINITE) };
        if wait_result != WAIT_OBJECT_0 {
            break;
        }

        let app_handle_for_main_thread = app_handle.clone();
        if let Err(error) = app_handle.run_on_main_thread(move || {
            crate::infrastructure::tauri::frontend_dev::reload_all_webviews(
                &app_handle_for_main_thread,
            );
        }) {
            eprintln!("failed to dispatch reload frontend request: {error}");
            break;
        }
    }
}

#[cfg(target_os = "windows")]
fn create_show_settings_event() -> Result<OwnedKernelHandle, String> {
    let event = unsafe { CreateEventW(None, false, false, SHOW_SETTINGS_EVENT_NAME) }
        .map_err(|error| format!("无法创建 Dasktop 二次启动通知事件：{error}"))?;

    Ok(unsafe { OwnedKernelHandle::new(event) })
}

#[cfg(target_os = "windows")]
fn create_reload_frontend_event() -> Result<OwnedKernelHandle, String> {
    let event = unsafe { CreateEventW(None, false, false, RELOAD_FRONTEND_EVENT_NAME) }
        .map_err(|error| format!("无法创建 Dasktop 热更重载通知事件：{error}"))?;

    Ok(unsafe { OwnedKernelHandle::new(event) })
}

/// Win32 内核句柄可在线程间转移所有权；这里用小包装集中说明 unsafe 边界。
#[cfg(target_os = "windows")]
struct OwnedKernelHandle {
    handle: Owned<HANDLE>,
}

#[cfg(target_os = "windows")]
impl OwnedKernelHandle {
    /// 接管 Win32 API 返回的拥有型 HANDLE，释放逻辑交给 windows-core 的 Owned。
    unsafe fn new(handle: HANDLE) -> Self {
        Self {
            handle: Owned::new(handle),
        }
    }

    /// 等待和置位事件只需要临时借用原始句柄，调用方不能关闭它。
    fn get(&self) -> HANDLE {
        *self.handle
    }
}

// SAFETY: 该包装始终独占一个内核 HANDLE，跨线程移动后仍只由拥有者在 Drop 时关闭。
#[cfg(target_os = "windows")]
unsafe impl Send for OwnedKernelHandle {}

// SAFETY: 等待、置位和关闭 HANDLE 都由操作系统同步；共享引用不会暴露 Rust 可变状态。
#[cfg(target_os = "windows")]
unsafe impl Sync for OwnedKernelHandle {}
