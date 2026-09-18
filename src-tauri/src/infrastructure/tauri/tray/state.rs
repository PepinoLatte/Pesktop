//! 托盘跨入口共享状态。

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::CheckMenuItem;
use tauri::Wry;

/// 托盘菜单中需要跨入口同步的可变控件状态。
pub(super) struct AppTrayState {
    pub(super) autostart_item: CheckMenuItem<Wry>,
    /// 托盘退出会真正销毁所有窗口，此标记用于区分系统关闭设置窗和应用退出。
    is_graceful_exit_requested: AtomicBool,
}

impl AppTrayState {
    /// 创建托盘状态，默认不处于主动退出流程。
    pub(super) fn new(autostart_item: CheckMenuItem<Wry>) -> Self {
        Self {
            autostart_item,
            is_graceful_exit_requested: AtomicBool::new(false),
        }
    }

    /// 标记托盘“关闭”触发的主动退出，设置窗关闭拦截会据此放行。
    pub(super) fn request_graceful_exit(&self) {
        self.is_graceful_exit_requested
            .store(true, Ordering::SeqCst);
    }

    /// 判断当前是否处于主动退出流程。
    pub(super) fn is_graceful_exit_requested(&self) -> bool {
        self.is_graceful_exit_requested.load(Ordering::SeqCst)
    }
}
