//! Windows Shell 文件能力封装，集中处理系统默认打开、回收站和快捷方式创建。

mod execute;
mod operation;
mod shortcut;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

pub use execute::{
    open_folder_in_explorer, open_item_with_system_default, open_parsing_name_with_system_default,
};
pub use operation::{recycle_path, recycle_paths};
pub use shortcut::create_shortcut_for_path;

pub(crate) use operation::move_paths_with_shell;

/// Shell 文件操作所属窗口类型，当前传输逻辑保留 owner 以便后续接回 Shell 交互。
#[cfg(target_os = "windows")]
pub(crate) type ShellOperationOwner = HWND;

/// 非 Windows 平台没有 Explorer Shell owner，使用空类型保持服务层签名稳定。
#[cfg(not(target_os = "windows"))]
pub(crate) type ShellOperationOwner = ();
