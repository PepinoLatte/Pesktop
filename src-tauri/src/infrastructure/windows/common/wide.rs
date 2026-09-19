//! Windows API 字符串转换工具集中在这里，避免各 Shell 模块重复维护 UTF-16 结尾规则。

use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

/// 将普通字符串转成 Windows API 需要的 UTF-16 零结尾缓冲区。
#[cfg(target_os = "windows")]
pub(crate) fn null_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

/// 将路径转成 Windows API 需要的 UTF-16 零结尾缓冲区。
#[cfg(target_os = "windows")]
pub(crate) fn path_null_terminated(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// 将单个路径转成 SHFileOperation 接受的双零结尾路径列表。
#[cfg(target_os = "windows")]
pub(crate) fn double_null_single_path(path: &Path) -> Vec<u16> {
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide.push(0);
    wide.push(0);
    wide
}

/// 将多个路径转成 SHFileOperation 接受的双零结尾路径列表。
#[cfg(target_os = "windows")]
pub(crate) fn double_null_path_list(paths: &[std::path::PathBuf]) -> Vec<u16> {
    double_null_paths(paths.iter().map(std::path::PathBuf::as_path))
}

/// 将路径迭代器转成 SHFileOperation 接受的双零结尾路径列表，避免调用方为了适配切片而克隆路径。
#[cfg(target_os = "windows")]
pub(crate) fn double_null_paths<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Vec<u16> {
    let mut wide = Vec::new();

    for path in paths {
        wide.extend(path.as_os_str().encode_wide());
        wide.push(0);
    }
    wide.push(0);

    wide
}

/// 从 Windows API 填充的 UTF-16 缓冲区解析 Rust 字符串，截取至首个空字符。
#[cfg(target_os = "windows")]
pub(crate) fn from_null_terminated_u16(slice: &[u16]) -> String {
    let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..len])
}
