//! Windows 原生文件夹选择器封装，保持设置页选择收纳目录时的系统一致性。

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

#[cfg(target_os = "windows")]
const HRESULT_ERROR_CANCELLED: i32 = 0x800704C7_u32 as i32;

/// 文件夹选择器窗口所属句柄类型，命令层负责从 Tauri 窗口中提取。
#[cfg(target_os = "windows")]
pub(crate) type FolderDialogOwner = HWND;

/// 非 Windows 平台没有当前产品目标里的原生文件夹选择语义，保留同名类型简化调用方分支。
#[cfg(not(target_os = "windows"))]
pub(crate) type FolderDialogOwner = ();

/// 文件夹选择器必须使用 Windows Shell 原生对话框，保证用户看到熟悉的目录选择体验。
#[cfg(target_os = "windows")]
pub fn choose_collection_root_folder(owner: FolderDialogOwner) -> Result<Option<String>, String> {
    use crate::infrastructure::windows::common::com;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::Ole::OleInitialize;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    unsafe {
        let _ = OleInitialize(None);
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("无法创建文件夹选择器：{error}"))?;
        dialog
            .SetOptions(FOS_PICKFOLDERS)
            .map_err(|error| format!("无法设置文件夹选择模式：{error}"))?;

        if let Err(error) = dialog.Show(Some(owner)) {
            if error.code().0 == HRESULT_ERROR_CANCELLED {
                return Ok(None);
            }

            return Err(format!("无法打开文件夹选择器：{error}"));
        }

        let item = dialog
            .GetResult()
            .map_err(|error| format!("无法读取选择的文件夹：{error}"))?;
        let path = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| format!("无法解析选择的文件夹路径：{error}"))?;
        let path_text = com::take_pwstr_string(path)
            .map_err(|error| format!("无法转换选择的文件夹路径：{error}"))?;

        Ok(Some(path_text))
    }
}

/// 非 Windows 平台没有当前产品目标里的原生文件夹选择语义，保持显式错误避免误判已支持。
#[cfg(not(target_os = "windows"))]
pub fn choose_collection_root_folder(_owner: FolderDialogOwner) -> Result<Option<String>, String> {
    Err("当前平台暂不支持选择 Box 收纳位置".to_string())
}
