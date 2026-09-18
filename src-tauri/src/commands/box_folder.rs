use crate::domain::desktop::{BoxConflictPolicy, BoxDeletePolicy, BoxDropAction};
use crate::services::box_folder;

/// 打开系统文件夹选择器，选择后续新建 Box 使用的收纳根目录。
#[tauri::command]
pub fn choose_collection_root_folder(
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let owner = window
            .hwnd()
            .map_err(|error| format!("无法获取设置窗口句柄：{error}"))?;

        box_folder::choose_collection_root_folder(owner)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;

        box_folder::choose_collection_root_folder(())
    }
}

/// 在用户设置的收纳根目录下创建单个 Box 的真实文件夹。
#[tauri::command]
pub fn create_box_folder(root_path: String, folder_name: String) -> Result<String, String> {
    box_folder::create_box_folder(&root_path, &folder_name)
}

/// 按当前删除策略处理 Box 真实文件夹；成功后前端才会删除 Box 记录。
#[tauri::command]
pub fn delete_box_folder(
    folder_path: String,
    desktop_path: String,
    policy: String,
    conflict_policy: String,
) -> Result<(), String> {
    box_folder::delete_box_folder(
        &folder_path,
        &desktop_path,
        policy.parse::<BoxDeletePolicy>()?,
        conflict_policy.parse::<BoxConflictPolicy>()?,
    )
}

/// 把单个 Box 的真实文件夹移动到当前收纳根目录，成功后返回新的文件夹路径。
#[tauri::command]
pub fn migrate_box_folder(folder_path: String, root_path: String) -> Result<String, String> {
    box_folder::migrate_box_folder(&folder_path, &root_path)
}

/// 用系统 Explorer 打开 Box 对应的真实文件夹。
#[tauri::command]
pub fn open_box_folder(folder_path: String) -> Result<(), String> {
    box_folder::open_box_folder(&folder_path)
}

/// 按当前拖入策略把外部文件复制、移动或映射到 Box 真实文件夹。
#[tauri::command]
pub fn handle_box_dropped_paths(
    folder_path: String,
    paths: Vec<String>,
    action: String,
    conflict_policy: String,
) -> Result<Vec<String>, String> {
    box_folder::handle_box_dropped_paths(
        &folder_path,
        &paths,
        action.parse::<BoxDropAction>()?,
        conflict_policy.parse::<BoxConflictPolicy>()?,
    )
}

/// 按当前拖出策略把 Box 内文件复制、移动或映射到 Windows 桌面目录。
#[tauri::command]
pub fn handle_box_dragged_paths_to_desktop(
    desktop_path: String,
    paths: Vec<String>,
    action: String,
    conflict_policy: String,
) -> Result<(), String> {
    box_folder::handle_box_dragged_paths_to_desktop(
        &desktop_path,
        &paths,
        action.parse::<BoxDropAction>()?,
        conflict_policy.parse::<BoxConflictPolicy>()?,
    )
}
