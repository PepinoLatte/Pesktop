//! Windows 全局内存写入封装，供剪贴板和拖放等 Shell 互操作复用。

#[cfg(target_os = "windows")]
use std::ptr;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::GlobalFree;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HGLOBAL;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

/// 创建可交给 Shell 接管的可移动全局内存，并写入指定字节内容。
#[cfg(target_os = "windows")]
pub(crate) fn create_moveable_from_bytes(bytes: &[u8], message: &str) -> Result<HGLOBAL, String> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }
        .map_err(|error| format!("{message}：{error}"))?;
    let pointer = unsafe { GlobalLock(handle) }.cast::<u8>();
    if pointer.is_null() {
        return Err(format!("{message}：无法锁定全局内存"));
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
        let _ = GlobalUnlock(handle);
    }

    Ok(handle)
}

/// 释放尚未成功交给 Windows Shell 接管的全局内存，避免剪贴板写入失败时泄漏句柄。
#[cfg(target_os = "windows")]
pub(crate) unsafe fn free_if_unclaimed(handle: HGLOBAL) {
    if handle.is_invalid() {
        return;
    }

    let _ = GlobalFree(Some(handle));
}
