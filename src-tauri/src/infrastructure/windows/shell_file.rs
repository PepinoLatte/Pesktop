//! Windows Shell 文件能力封装，集中处理系统默认打开、回收站和快捷方式创建。

use std::path::{Path, PathBuf};

#[cfg(not(target_os = "windows"))]
use std::fs;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

/// Shell 文件操作所属窗口类型，当前传输逻辑保留 owner 以便后续接回 Shell 交互。
#[cfg(target_os = "windows")]
pub(crate) type ShellOperationOwner = HWND;

/// 非 Windows 平台没有 Explorer Shell owner，使用空类型保持服务层签名稳定。
#[cfg(not(target_os = "windows"))]
pub(crate) type ShellOperationOwner = ();

/// 使用系统默认程序打开文件夹，方便用户检查 Box 对应的磁盘位置。
pub fn open_folder_in_explorer(folder_path: &str) -> Result<(), String> {
    let folder = Path::new(folder_path);
    if !folder.exists() {
        return Err("Box 文件夹不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(folder)
}

/// 使用系统默认程序打开 Box 文件项，前端不需要理解具体文件类型。
pub fn open_item_with_system_default(path: &str) -> Result<(), String> {
    let item_path = Path::new(path);
    if !item_path.exists() {
        return Err("文件项不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(item_path)
}

/// 使用 Windows Shell 解析名打开虚拟项，供“此电脑、回收站”等没有真实路径的对象复用。
pub fn open_parsing_name_with_system_default(parsing_name: &str) -> Result<(), String> {
    open_shell_parsing_name_with_system_default(parsing_name)
}

/// 删除单个路径时进入回收站，保留 Windows 侧恢复能力。
pub fn recycle_path(path: &Path) -> Result<(), String> {
    recycle_paths(&[path.to_path_buf()])
}

/// 删除多个路径时进入回收站，避免自绘文件区绕过系统恢复能力。
#[cfg(target_os = "windows")]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FO_DELETE, SHFILEOPSTRUCTW};

    let mut from = to_double_null_path_list(paths);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_mut_ptr()),
        fFlags: FOF_ALLOWUNDO.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };

    if result != 0 {
        return Err(format!(
            "Windows 无法将 Box 文件项移入回收站，错误码 {result}"
        ));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("已取消删除 Box 文件项".to_string());
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

/// 非 Windows 平台没有回收站语义，删除文件夹时退回直接删除以便开发调试。
#[cfg(not(target_os = "windows"))]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        recycle_path_without_shell(path)?;
    }

    Ok(())
}

/// 为真实路径创建 `.lnk` 映射快捷方式，保持 Box “映射”语义不复制原文件。
#[cfg(target_os = "windows")]
pub fn create_shortcut_for_path(path: &Path, shortcut_path: &Path) -> Result<(), String> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::Ole::OleInitialize;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    if shortcut_path.exists() {
        return Err("目标快捷方式已经存在，已停止映射".to_string());
    }

    let target_wide = to_wide_path(path);
    let shortcut_wide = to_wide_path(shortcut_path);

    unsafe {
        let _ = OleInitialize(None);
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("无法创建映射快捷方式：{error}"))?;
        shell_link
            .SetPath(PCWSTR(target_wide.as_ptr()))
            .map_err(|error| format!("无法写入映射目标：{error}"))?;
        if let Some(parent) = path.parent() {
            let working_directory = to_wide_path(parent);
            shell_link
                .SetWorkingDirectory(PCWSTR(working_directory.as_ptr()))
                .map_err(|error| format!("无法写入映射工作目录：{error}"))?;
        }

        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
        persist_file
            .Save(PCWSTR(shortcut_wide.as_ptr()), true)
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
    }

    Ok(())
}

/// 非 Windows 平台没有当前产品目标里的 `.lnk` 映射语义，保持显式错误避免误判已支持。
#[cfg(not(target_os = "windows"))]
pub fn create_shortcut_for_path(_path: &Path, _shortcut_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持创建 Box 映射快捷方式".to_string())
}

#[cfg(target_os = "windows")]
fn open_path_with_system_default(path: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path_wide = path
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(format!(
            "系统无法打开 Box 文件夹，错误码 {}",
            result.0 as isize
        ));
    }

    Ok(())
}

/// ShellExecute 可直接接收 `::{GUID}` 和 Known Folder 路径，保持与 Explorer 双击语义一致。
#[cfg(target_os = "windows")]
fn open_shell_parsing_name_with_system_default(parsing_name: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let parsing_name_wide = parsing_name
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation_wide.as_ptr()),
            PCWSTR(parsing_name_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(format!(
            "系统无法打开该系统桌面项目，错误码 {}",
            result.0 as isize
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_path_with_system_default(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开 Box 文件夹".to_string())
}

/// 非 Windows 平台没有 Shell 虚拟桌面项，保持显式错误避免前端误判已支持。
#[cfg(not(target_os = "windows"))]
fn open_shell_parsing_name_with_system_default(_parsing_name: &str) -> Result<(), String> {
    Err("当前平台暂不支持打开系统桌面项目".to_string())
}

#[cfg(target_os = "windows")]
fn move_paths_to_shared_folder_with_shell(
    moves: &[(PathBuf, PathBuf)],
    destination_folder: &Path,
) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_NOCONFIRMMKDIR, FO_MOVE, SHFILEOPSTRUCTW,
    };

    let sources = moves
        .iter()
        .map(|(source, _)| source.clone())
        .collect::<Vec<_>>();
    let mut from = to_double_null_path_list(&sources);
    let mut to = to_double_null_single_path(destination_folder);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_MOVE,
        pFrom: PCWSTR(from.as_mut_ptr()),
        pTo: PCWSTR(to.as_mut_ptr()),
        fFlags: FOF_NOCONFIRMMKDIR.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };

    if result != 0 {
        return Err(format!("Windows 无法移动 Box 文件项，错误码 {result}"));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("已取消移动 Box 文件项".to_string());
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn move_single_path_with_shell(source: &Path, destination: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_NOCONFIRMMKDIR, FO_MOVE, SHFILEOPSTRUCTW,
    };

    let mut from = to_double_null_single_path(source);
    let mut to = to_double_null_single_path(destination);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_MOVE,
        pFrom: PCWSTR(from.as_mut_ptr()),
        pTo: PCWSTR(to.as_mut_ptr()),
        fFlags: FOF_NOCONFIRMMKDIR.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };

    if result != 0 {
        return Err(format!("Windows 无法移动 Box 文件项，错误码 {result}"));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("已取消移动 Box 文件项".to_string());
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

#[cfg(target_os = "windows")]
fn to_double_null_single_path(path: &Path) -> Vec<u16> {
    let mut wide = path.to_string_lossy().encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide.push(0);
    wide
}

#[cfg(target_os = "windows")]
fn to_double_null_path_list(paths: &[PathBuf]) -> Vec<u16> {
    let mut wide = Vec::new();

    for path in paths {
        wide.extend(path.to_string_lossy().encode_utf16());
        wide.push(0);
    }
    wide.push(0);

    wide
}

#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
