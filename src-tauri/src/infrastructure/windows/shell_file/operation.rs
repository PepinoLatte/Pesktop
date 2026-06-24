//! Shell 文件操作逻辑，负责回收站删除和 Explorer 感知的文件移动。

use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use crate::infrastructure::windows::common::wide;
#[cfg(not(target_os = "windows"))]
use std::fs;

/// 删除单个路径时进入回收站，保留 Windows 侧恢复能力。
pub fn recycle_path(path: &Path) -> Result<(), String> {
    recycle_paths(&[path.to_path_buf()])
}

/// 删除多个路径时进入回收站，避免自绘文件区绕过系统恢复能力。
#[cfg(target_os = "windows")]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    use windows::Win32::UI::Shell::{FOF_ALLOWUNDO, FO_DELETE};

    run_shell_operation(
        FO_DELETE,
        wide::double_null_path_list(paths),
        None,
        FOF_ALLOWUNDO,
        ShellFileOperationMessages {
            failed: "Windows 无法将 Box 文件项移入回收站",
            aborted: "已取消删除 Box 文件项",
        },
    )
}

/// 非 Windows 平台没有回收站语义，删除文件夹时退回直接删除以便开发调试。
#[cfg(not(target_os = "windows"))]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        recycle_path_without_shell(path)?;
    }

    Ok(())
}

/// 使用 Windows Shell 移动真实文件路径，让 Explorer 桌面视图立即收到标准文件变更事件。
#[cfg(target_os = "windows")]
pub(crate) fn move_paths_with_shell(moves: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    if moves.is_empty() {
        return Ok(());
    }

    if let Some(shared_destination_folder) = resolve_shared_plain_move_destination(moves) {
        return move_paths_to_shared_folder_with_shell(moves, &shared_destination_folder);
    }

    for (source, destination) in moves {
        move_single_path_with_shell(source, destination)?;
    }

    Ok(())
}

/// 非 Windows 平台没有 Explorer Shell 移动能力，保持显式错误避免误以为桌面刷新语义一致。
#[cfg(not(target_os = "windows"))]
pub(crate) fn move_paths_with_shell(_moves: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    Err("当前平台暂不支持使用系统 Shell 移动 Box 文件项".to_string())
}

#[cfg(target_os = "windows")]
fn move_paths_to_shared_folder_with_shell(
    moves: &[(PathBuf, PathBuf)],
    destination_folder: &Path,
) -> Result<(), String> {
    run_shell_move(
        wide::double_null_paths(moves.iter().map(|(source, _)| source.as_path())),
        wide::double_null_single_path(destination_folder),
    )
}

#[cfg(target_os = "windows")]
fn move_single_path_with_shell(source: &Path, destination: &Path) -> Result<(), String> {
    run_shell_move(
        wide::double_null_single_path(source),
        wide::double_null_single_path(destination),
    )
}

#[cfg(target_os = "windows")]
fn run_shell_move(from: Vec<u16>, to: Vec<u16>) -> Result<(), String> {
    use windows::Win32::UI::Shell::{FOF_NOCONFIRMMKDIR, FO_MOVE};

    run_shell_operation(
        FO_MOVE,
        from,
        Some(to),
        FOF_NOCONFIRMMKDIR,
        ShellFileOperationMessages {
            failed: "Windows 无法移动 Box 文件项",
            aborted: "已取消移动 Box 文件项",
        },
    )
}

#[cfg(target_os = "windows")]
struct ShellFileOperationMessages {
    failed: &'static str,
    aborted: &'static str,
}

#[cfg(target_os = "windows")]
fn run_shell_operation(
    operation_kind: u32,
    mut from: Vec<u16>,
    to: Option<Vec<u16>>,
    flags: windows::Win32::UI::Shell::FILEOPERATION_FLAGS,
    messages: ShellFileOperationMessages,
) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHFileOperationW, SHFILEOPSTRUCTW};

    let mut to = to;
    let to_pointer = to
        .as_mut()
        .map(|value| PCWSTR(value.as_mut_ptr()))
        .unwrap_or_else(PCWSTR::null);

    // SHFileOperationW 需要双空结尾的 UTF-16 缓冲区在调用期间保持存活，故由本函数接管 Vec 生命周期。
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: operation_kind,
        pFrom: PCWSTR(from.as_mut_ptr()),
        pTo: to_pointer,
        fFlags: flags.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };

    if result != 0 {
        return Err(format!("{}，错误码 {result}", messages.failed));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err(messages.aborted.to_string());
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn resolve_shared_plain_move_destination(moves: &[(PathBuf, PathBuf)]) -> Option<PathBuf> {
    let first_destination_folder = moves.first()?.1.parent()?.to_path_buf();
    for (source, destination) in moves {
        if destination.parent()? != first_destination_folder {
            return None;
        }
        if source.file_name()? != destination.file_name()? {
            return None;
        }
    }

    Some(first_destination_folder)
}

#[cfg(not(target_os = "windows"))]
fn recycle_path_without_shell(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("无法删除 Box 文件夹：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("无法删除 Box 文件项：{error}"))
    }
}
