//! ShellExecute 打开逻辑，统一真实路径和 Shell 解析名的系统默认打开行为。

use std::path::Path;

#[cfg(target_os = "windows")]
use crate::infrastructure::windows::common::wide;

/// ShellExecute 返回值大于 32 才代表成功，低值是 Win32 兼容错误码。
#[cfg(target_os = "windows")]
const SHELL_EXECUTE_SUCCESS_THRESHOLD: isize = 32;
/// ShellExecute 的打开动作动词，使用命名常量避免散落 `open` 字面量。
#[cfg(target_os = "windows")]
const SHELL_OPEN_VERB: &str = "open";

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

#[cfg(target_os = "windows")]
fn open_path_with_system_default(path: &Path) -> Result<(), String> {
    let path_wide = wide::path_null_terminated(path);

    shell_execute_open(
        windows::core::PCWSTR(path_wide.as_ptr()),
        "系统无法打开 Box 文件夹",
    )
}

/// ShellExecute 可直接接收 `::{GUID}` 和 Known Folder 路径，保持与 Explorer 双击语义一致。
#[cfg(target_os = "windows")]
fn open_shell_parsing_name_with_system_default(parsing_name: &str) -> Result<(), String> {
    let parsing_name_wide = wide::null_terminated(parsing_name);

    shell_execute_open(
        windows::core::PCWSTR(parsing_name_wide.as_ptr()),
        "系统无法打开该系统桌面项目",
    )
}

#[cfg(target_os = "windows")]
fn shell_execute_open(target: windows::core::PCWSTR, error_message: &str) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation_wide = wide::null_terminated(SHELL_OPEN_VERB);
    let result = unsafe {
        ShellExecuteW(
            None,
            windows::core::PCWSTR(operation_wide.as_ptr()),
            target,
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= SHELL_EXECUTE_SUCCESS_THRESHOLD {
        return Err(format!("{error_message}，错误码 {}", result.0 as isize));
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
