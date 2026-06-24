//! Windows 基础设施错误辅助函数，集中处理 `io::Error` 和系统错误码包装。

#[cfg(target_os = "windows")]
use std::io;

/// 将 Windows crate 错误转成 `io::Error::other`，避免每个调用点重复构造 Other 类型。
#[cfg(target_os = "windows")]
pub(crate) fn io_other(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

/// 携带调用点生成最新 Windows 错误，便于定位具体 API 失败原因。
#[cfg(target_os = "windows")]
pub(crate) fn last_os_error(operation: &str) -> io::Error {
    let code = unsafe { windows::Win32::Foundation::GetLastError() };
    io::Error::other(format!("{operation} failed with Windows error {}", code.0))
}
