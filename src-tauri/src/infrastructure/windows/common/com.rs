//! COM 分配资源释放工具，集中处理 Shell API 返回的 `CoTaskMemFree` 所有权。

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(target_os = "windows")]
use windows_core::PWSTR;

/// 将 Shell 返回的 `PWSTR` 转成 Rust 字符串，并立刻释放 COM 分配内存。
#[cfg(target_os = "windows")]
pub(crate) unsafe fn take_pwstr_string(
    value: PWSTR,
) -> Result<String, std::string::FromUtf16Error> {
    let result = value.to_string();
    free_cotaskmem_ptr(value.as_ptr());

    result
}

/// 释放 Shell/COM 通过任务内存分配器返回的裸指针；空指针保持幂等。
#[cfg(target_os = "windows")]
pub(crate) unsafe fn free_cotaskmem_ptr<T>(ptr: *const T) {
    if ptr.is_null() {
        return;
    }

    CoTaskMemFree(Some(ptr.cast::<c_void>()));
}
