//! Box 真实文件夹基础设施封装目录创建、迁移、删除和同目录文件重命名。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::desktop::{BoxConflictPolicy, BoxDeletePolicy, BoxDropAction};
use crate::infrastructure::filesystem::{naming, transfer};
use crate::infrastructure::windows::shell_file;

/// 在收纳根目录下创建单个 Box 对应的真实文件夹，物理目录名由业务 ID 派生而非用户标题派生。
pub(crate) fn create_box_folder(root_path: &str, folder_name: &str) -> Result<String, String> {
    let root = ensure_collection_root(root_path)?;
    let sanitized_folder_name = naming::sanitize_box_folder_name(folder_name)?;
    let folder_path = root.join(sanitized_folder_name);

    fs::create_dir(&folder_path).map_err(|error| {
        format!(
            "无法创建 Box 文件夹 {}：{error}",
            folder_path.to_string_lossy()
        )
    })?;

    Ok(folder_path.to_string_lossy().to_string())
}

/// 删除 Box 前按策略处理真实文件夹；调用方只有在本函数成功后才能删除数据库记录。
pub(crate) fn delete_box_folder(
    folder_path: &str,
    desktop_path: &str,
    policy: BoxDeletePolicy,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let folder = PathBuf::from(folder_path);
    if !folder.exists() {
        return Ok(());
    }
    if !folder.is_dir() {
        return Err("Box 路径不是文件夹，已停止删除以避免误操作".to_string());
    }

    match policy {
        BoxDeletePolicy::KeepFolder => Ok(()),
        BoxDeletePolicy::MoveContentsToDesktop => {
            move_folder_contents_to_desktop(&folder, desktop_path, conflict_policy)
        }
        BoxDeletePolicy::RecycleFolder => shell_file::recycle_path(&folder),
    }
}

/// 将单个 Box 的真实文件夹迁移到新的收纳根目录，保留物理目录名和 Box 展示标题的解耦关系。
pub(crate) fn migrate_box_folder(folder_path: &str, root_path: &str) -> Result<String, String> {
    let source = PathBuf::from(folder_path);
    if !source.exists() {
        return Err(format!(
            "Box 文件夹不存在，无法迁移：{}",
            source.to_string_lossy()
        ));
    }
    if !source.is_dir() {
        return Err("Box 路径不是文件夹，已停止迁移以避免误操作".to_string());
    }

    let root = ensure_collection_root(root_path)?;
    let folder_name = source
        .file_name()
        .ok_or_else(|| "无法解析 Box 文件夹名称".to_string())?;
    let destination = root.join(folder_name);
    if naming::paths_refer_to_same_entry(&source, &destination)? {
        return Ok(destination.to_string_lossy().to_string());
    }
    if destination.exists() {
        return Err(format!(
            "目标收纳位置已存在同名文件夹，已停止迁移：{}",
            destination.to_string_lossy()
        ));
    }
    if naming::destination_is_inside_source(&source, &root)? {
        return Err("不能把 Box 文件夹迁移到自身内部".to_string());
    }

    transfer::move_path_without_shell_prompt(&source, &destination)?;

    Ok(destination.to_string_lossy().to_string())
}

/// 将同一目录下的文件项重命名，避免前端拼接路径时跨目录移动真实文件。
pub(crate) fn rename_item(path: &str, new_name: &str) -> Result<String, String> {
    let source = PathBuf::from(path);
    if !source.exists() {
        return Err("文件项不存在，无法重命名".to_string());
    }

    let sanitized_name = naming::sanitize_user_file_name(new_name)?;
    let parent = source
        .parent()
        .ok_or_else(|| "无法解析文件所在目录，已停止重命名".to_string())?;
    let destination = parent.join(sanitized_name);
    if destination == source {
        return Ok(destination.to_string_lossy().to_string());
    }
    if destination.exists() {
        return Err("同名文件已经存在，已停止重命名".to_string());
    }

    fs::rename(&source, &destination).map_err(|error| format!("无法重命名文件项：{error}"))?;

    Ok(destination.to_string_lossy().to_string())
}

/// 按当前拖入策略处理外部文件路径；调用方只传真实文件系统路径，不接收 Shell 虚拟对象。
pub(crate) fn handle_box_dropped_paths(
    folder_path: &str,
    paths: &[String],
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<String>, String> {
    let folder = PathBuf::from(folder_path);
    if !folder.is_dir() {
        return Err("Box 文件夹不存在，无法处理拖入文件".to_string());
    }

    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Err("未读取到可处理的拖入文件".to_string());
    }

    transfer::transfer_paths_without_shell_prompts(&sources, &folder, action, conflict_policy).map(
        |destinations| {
            destinations
                .iter()
                .map(|destination| destination.to_string_lossy().to_string())
                .collect()
        },
    )
}

/// 按当前拖出策略处理 Box 内文件到桌面目录；桌面目录缺失时先创建以保持 Shell 语义稳定。
pub(crate) fn handle_box_dragged_paths_to_desktop(
    desktop_path: &str,
    paths: &[String],
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    if desktop_path.trim().is_empty() {
        return Err("无法定位桌面路径，已停止拖出文件".to_string());
    }

    let desktop = PathBuf::from(desktop_path);
    if !desktop.exists() {
        fs::create_dir_all(&desktop).map_err(|error| format!("无法创建桌面目录：{error}"))?;
    }
    if !desktop.is_dir() {
        return Err("桌面路径不是文件夹，无法处理拖出文件".to_string());
    }

    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Err("未读取到可拖出的 Box 文件".to_string());
    }

    transfer::transfer_paths_without_shell_prompts(&sources, &desktop, action, conflict_policy)
        .map(|_| ())
}

fn ensure_collection_root(root_path: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(root_path);
    if root_path.trim().is_empty() {
        return Err("请先选择 Box 收纳位置".to_string());
    }
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|error| format!("无法创建收纳根目录：{error}"))?;
    }
    if !root.is_dir() {
        return Err("收纳位置必须是文件夹".to_string());
    }

    Ok(root)
}

fn move_folder_contents_to_desktop(
    folder: &Path,
    desktop_path: &str,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let desktop = PathBuf::from(desktop_path);
    if desktop_path.trim().is_empty() {
        return Err("无法定位桌面路径，已停止删除 Box".to_string());
    }
    if !desktop.exists() {
        fs::create_dir_all(&desktop).map_err(|error| format!("无法创建桌面目录：{error}"))?;
    }

    let children = collect_folder_children(folder)?;
    if !children.is_empty() {
        transfer::transfer_paths_without_shell_prompts(
            &children,
            &desktop,
            BoxDropAction::Move,
            conflict_policy,
        )
        .map(|_| ())?;
    }

    match fs::remove_dir(folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("收纳文件夹未清空，已保留 Box 记录：{error}")),
    }
}

fn collect_folder_children(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut children = Vec::new();

    for entry in fs::read_dir(folder).map_err(|error| format!("无法读取 Box 文件夹：{error}"))?
    {
        children.push(
            entry
                .map_err(|error| format!("无法读取 Box 文件夹内容：{error}"))?
                .path(),
        );
    }

    Ok(children)
}
