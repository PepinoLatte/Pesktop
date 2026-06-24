//! Windows API 字符串转换工具集中在这里，避免各 Shell 模块重复维护 UTF-16 结尾规则。

use std::path::Path;

/// 将普通字符串转成 Windows API 需要的 UTF-16 零结尾缓冲区。
#[cfg(target_os = "windows")]
pub(crate) fn null_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

/// 将路径转成 Windows API 需要的 UTF-16 零结尾缓冲区。
#[cfg(target_os = "windows")]
pub(crate) fn path_null_terminated(path: &Path) -> Vec<u16> {
    null_terminated(&path.to_string_lossy())
}

/// 将单个路径转成 SHFileOperation 接受的双零结尾路径列表。
#[cfg(target_os = "windows")]
pub(crate) fn double_null_single_path(path: &Path) -> Vec<u16> {
    let mut wide = path.to_string_lossy().encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide.push(0);
    wide
}

/// 将多个路径转成 SHFileOperation 接受的双零结尾路径列表。
#[cfg(target_os = "windows")]
pub(crate) fn double_null_path_list(paths: &[std::path::PathBuf]) -> Vec<u16> {
    let mut wide = Vec::new();

    for path in paths {
        wide.extend(path.to_string_lossy().encode_utf16());
        wide.push(0);
    }
    wide.push(0);

    wide
}
