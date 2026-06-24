//! Windows 文件剪贴板封装，负责与 Explorer 兼容的 CF_HDROP 路径列表和 DropEffect 意图。

use std::path::PathBuf;

use crate::domain::desktop::FileClipboardOperation;

/// 从系统剪贴板读取到的文件路径和用户操作意图。
pub(crate) struct FileClipboardPayload {
    pub(crate) operation: FileClipboardOperation,
    pub(crate) paths: Vec<PathBuf>,
}

/// 将真实文件路径写入系统文件剪贴板，供 Box 或 Explorer 后续粘贴。
#[cfg(target_os = "windows")]
pub(crate) fn write_file_list(
    paths: &[PathBuf],
    operation: FileClipboardOperation,
) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }

    let _clipboard = ClipboardSession::open()?;
    unsafe {
        EmptyClipboard().map_err(|error| format!("无法清空系统剪贴板：{error}"))?;
    }

    let hdrop_memory = create_hdrop_memory(paths)?;
    unsafe {
        SetClipboardData(u32::from(CF_HDROP.0), Some(HANDLE(hdrop_memory.0)))
            .map_err(|error| format!("无法写入文件剪贴板路径列表：{error}"))?;
    }

    let effect_memory = create_drop_effect_memory(operation)?;
    unsafe {
        SetClipboardData(
            preferred_drop_effect_format()?,
            Some(HANDLE(effect_memory.0)),
        )
        .map_err(|error| format!("无法写入文件剪贴板操作意图：{error}"))?;
    }

    Ok(())
}

/// 读取系统文件剪贴板；没有文件列表时返回 None，避免 Ctrl+V 在空剪贴板时报错。
#[cfg(target_os = "windows")]
pub(crate) fn read_file_list() -> Result<Option<FileClipboardPayload>, String> {
    let _clipboard = ClipboardSession::open()?;
    if unsafe { IsClipboardFormatAvailable(u32::from(CF_HDROP.0)) }.is_err() {
        return Ok(None);
    }

    let handle = unsafe { GetClipboardData(u32::from(CF_HDROP.0)) }
        .map_err(|error| format!("无法读取文件剪贴板路径列表：{error}"))?;
    let hdrop = HDROP(handle.0);
    if hdrop.is_invalid() {
        return Ok(None);
    }

    let paths = read_hdrop_paths(hdrop)?;
    if paths.is_empty() {
        return Ok(None);
    }

    Ok(Some(FileClipboardPayload {
        operation: read_preferred_operation().unwrap_or(FileClipboardOperation::Copy),
        paths,
    }))
}

/// 非 Windows 平台没有 Explorer 文件剪贴板语义，保持显式错误便于开发时定位平台差异。
#[cfg(not(target_os = "windows"))]
pub(crate) fn write_file_list(
    _paths: &[PathBuf],
    _operation: FileClipboardOperation,
) -> Result<(), String> {
    Err("当前平台暂不支持 Windows 文件剪贴板".to_string())
}

/// 非 Windows 平台粘贴时安静返回空剪贴板，避免开发环境误触真实文件操作。
#[cfg(not(target_os = "windows"))]
pub(crate) fn read_file_list() -> Result<Option<FileClipboardPayload>, String> {
    Ok(None)
}

#[cfg(target_os = "windows")]
use std::mem::size_of;
#[cfg(target_os = "windows")]
use windows::core::w;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HANDLE, HGLOBAL, POINT};
#[cfg(target_os = "windows")]
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
#[cfg(target_os = "windows")]
use windows::Win32::System::Ole::{CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_MOVE};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{DragQueryFileW, DROPFILES, HDROP};
#[cfg(target_os = "windows")]
use windows_core::BOOL;

#[cfg(target_os = "windows")]
use crate::infrastructure::windows::common::global_memory;

#[cfg(target_os = "windows")]
struct ClipboardSession;

#[cfg(target_os = "windows")]
impl ClipboardSession {
    /// 打开系统剪贴板后用 Drop 自动关闭，防止异常返回时占住剪贴板。
    fn open() -> Result<Self, String> {
        unsafe {
            OpenClipboard(None).map_err(|error| format!("无法打开系统剪贴板：{error}"))?;
        }

        Ok(Self)
    }
}

#[cfg(target_os = "windows")]
impl Drop for ClipboardSession {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
fn create_hdrop_memory(paths: &[PathBuf]) -> Result<HGLOBAL, String> {
    let mut bytes = Vec::new();
    let header = DROPFILES {
        pFiles: size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: BOOL(0),
        fWide: BOOL(1),
    };
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            (&header as *const DROPFILES).cast::<u8>(),
            size_of::<DROPFILES>(),
        )
    };
    bytes.extend_from_slice(header_bytes);

    for path in paths {
        bytes.extend(
            path.to_string_lossy()
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        bytes.extend(0u16.to_le_bytes());
    }
    bytes.extend(0u16.to_le_bytes());

    global_memory::create_moveable_from_bytes(&bytes, "无法创建文件剪贴板路径内存")
}

#[cfg(target_os = "windows")]
fn create_drop_effect_memory(operation: FileClipboardOperation) -> Result<HGLOBAL, String> {
    let effect = match operation {
        FileClipboardOperation::Copy => DROPEFFECT_COPY.0,
        FileClipboardOperation::Cut => DROPEFFECT_MOVE.0,
    };

    global_memory::create_moveable_from_bytes(&effect.to_le_bytes(), "无法创建文件剪贴板操作内存")
}

#[cfg(target_os = "windows")]
fn read_hdrop_paths(hdrop: HDROP) -> Result<Vec<PathBuf>, String> {
    let count = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    let mut paths = Vec::new();

    for index in 0..count {
        let length = unsafe { DragQueryFileW(hdrop, index, None) };
        if length == 0 {
            continue;
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { DragQueryFileW(hdrop, index, Some(&mut buffer)) };
        if copied == 0 {
            continue;
        }

        paths.push(PathBuf::from(String::from_utf16_lossy(
            &buffer[..copied as usize],
        )));
    }

    Ok(paths)
}

#[cfg(target_os = "windows")]
fn read_preferred_operation() -> Option<FileClipboardOperation> {
    let format = preferred_drop_effect_format().ok()?;
    if unsafe { IsClipboardFormatAvailable(format) }.is_err() {
        return None;
    }

    let handle = unsafe { GetClipboardData(format) }.ok()?;
    let hglobal = HGLOBAL(handle.0);
    if hglobal.is_invalid() || unsafe { GlobalSize(hglobal) } < size_of::<u32>() {
        return None;
    }

    let pointer = unsafe { GlobalLock(hglobal) }.cast::<u32>();
    if pointer.is_null() {
        return None;
    }
    let effect = unsafe { *pointer };
    unsafe {
        let _ = GlobalUnlock(hglobal);
    }

    if effect & DROPEFFECT_MOVE.0 == DROPEFFECT_MOVE.0 {
        Some(FileClipboardOperation::Cut)
    } else {
        Some(FileClipboardOperation::Copy)
    }
}

#[cfg(target_os = "windows")]
fn preferred_drop_effect_format() -> Result<u32, String> {
    let format = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
    if format == 0 {
        return Err("无法注册文件剪贴板操作格式".to_string());
    }

    Ok(format)
}
