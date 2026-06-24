//! HDROP 文件列表读写工具，供剪贴板和原生拖放共享 Windows Shell 路径协议。

use std::ffi::OsString;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{HGLOBAL, POINT};
use windows::Win32::UI::Shell::{DragQueryFileW, DROPFILES, HDROP};
use windows_core::BOOL;

use super::global_memory;

/// `DragQueryFileW` 用 u32::MAX 查询 HDROP 文件数量，集中命名避免剪贴板和拖放各写一份。
const HDROP_QUERY_FILE_COUNT_INDEX: u32 = u32::MAX;
/// UTF-16 路径列表中每个路径和整个列表都以 NUL 结尾。
const WIDE_NUL_TERMINATOR: u16 = 0;

/// 创建可交给剪贴板接管的 HDROP 内存块，保持与 Explorer 文件复制协议一致。
pub(crate) fn create_file_list_memory(
    paths: &[PathBuf],
    error_message: &str,
) -> Result<HGLOBAL, String> {
    let mut bytes = create_dropfiles_header_bytes();

    for path in paths {
        append_wide_path(&mut bytes, path);
    }
    append_wide_nul(&mut bytes);

    global_memory::create_moveable_from_bytes(&bytes, error_message)
}

/// 从 HDROP 中读取文件路径；读取失败的单项会被跳过，避免一个坏路径阻断整次拖放。
pub(crate) fn read_paths(hdrop: HDROP) -> Vec<PathBuf> {
    let count = unsafe { DragQueryFileW(hdrop, HDROP_QUERY_FILE_COUNT_INDEX, None) };
    let mut paths = Vec::new();

    for index in 0..count {
        let Some(path) = read_path_at(hdrop, index) else {
            continue;
        };
        paths.push(path);
    }

    paths
}

/// 读取 HDROP 并转成业务层使用的路径字符串，供原生拖放 payload 直接复用。
pub(crate) fn read_path_strings(hdrop: HDROP) -> Vec<String> {
    read_paths(hdrop)
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn create_dropfiles_header_bytes() -> Vec<u8> {
    let header = DROPFILES {
        pFiles: size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: BOOL(0),
        fWide: BOOL(1),
    };

    unsafe {
        std::slice::from_raw_parts(
            (&header as *const DROPFILES).cast::<u8>(),
            size_of::<DROPFILES>(),
        )
        .to_vec()
    }
}

fn append_wide_path(bytes: &mut Vec<u8>, path: &Path) {
    for value in path.as_os_str().encode_wide() {
        bytes.extend(value.to_le_bytes());
    }
    append_wide_nul(bytes);
}

fn append_wide_nul(bytes: &mut Vec<u8>) {
    bytes.extend(WIDE_NUL_TERMINATOR.to_le_bytes());
}

fn read_path_at(hdrop: HDROP, index: u32) -> Option<PathBuf> {
    let character_count = unsafe { DragQueryFileW(hdrop, index, None) };
    if character_count == 0 {
        return None;
    }

    let mut buffer = vec![WIDE_NUL_TERMINATOR; character_count as usize + 1];
    let copied = unsafe { DragQueryFileW(hdrop, index, Some(&mut buffer)) };
    if copied == 0 {
        return None;
    }

    Some(PathBuf::from(OsString::from_wide(
        &buffer[..copied as usize],
    )))
}
