//! 文件名、Box 物理目录名和冲突目标命名策略集中在这里，避免真实文件操作接收未校验路径片段。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::filesystem::WINDOWS_SHORTCUT_SUFFIX;

const BOX_FOLDER_PREFIX: &str = "box_";
/// 自动重命名最多尝试的副本序号上限，防止异常目录导致无界循环。
const DUPLICATE_NAME_LIMIT: usize = 10_000;

/// 校验由前端 Box ID 派生的物理目录名，避免用户标题参与真实路径拼接。
pub(crate) fn sanitize_box_folder_name(folder_name: &str) -> Result<String, String> {
    let trimmed = folder_name.trim();
    if !trimmed.starts_with(BOX_FOLDER_PREFIX) {
        return Err("Box 文件夹名必须由 Dasktop 生成".to_string());
    }
    if trimmed
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
    {
        return Ok(trimmed.to_string());
    }

    Err("Box 文件夹名包含非法字符".to_string())
}

/// 校验用户输入的同目录重命名目标，避免跨目录移动或 Windows 保留字符进入文件系统。
pub(crate) fn sanitize_user_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("文件名不能为空".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("文件名不能使用系统保留名称".to_string());
    }
    if trimmed
        .chars()
        .any(|value| matches!(value, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err("文件名包含 Windows 不允许的字符".to_string());
    }

    Ok(trimmed.trim_end_matches('.').trim_end().to_string())
}

/// 为映射模式生成 `.lnk` 目标路径，文件夹和无扩展名路径都使用稳定展示名兜底。
pub(crate) fn desired_shortcut_path(source: &Path, destination: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .or_else(|| source.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "映射项目".to_string());

    destination.join(format!("{stem}{WINDOWS_SHORTCUT_SUFFIX}"))
}

/// 重命名冲突目标时保持 Explorer 常见的 `名称 (2).ext` 形式。
pub(crate) fn resolve_renamed_destination(
    desired_destination: &Path,
    reserved_paths: &mut HashSet<String>,
) -> Result<PathBuf, String> {
    if !desired_destination.exists() && reserve_path(desired_destination, reserved_paths)? {
        return Ok(desired_destination.to_path_buf());
    }

    let parent = desired_destination
        .parent()
        .ok_or_else(|| "无法解析目标文件夹".to_string())?;
    let (stem, extension) = split_duplicate_name(desired_destination);

    for duplicate_index in 2..DUPLICATE_NAME_LIMIT {
        let candidate = parent.join(format!("{stem} ({duplicate_index}){extension}"));
        if !candidate.exists() && reserve_path(&candidate, reserved_paths)? {
            return Ok(candidate);
        }
    }

    Err("目标目录存在过多同名文件，无法自动生成新文件名".to_string())
}

/// 将路径登记到当前批次的保留集合，避免多来源解析到同一路径后互相覆盖。
pub(crate) fn reserve_path(
    path: &Path,
    reserved_paths: &mut HashSet<String>,
) -> Result<bool, String> {
    Ok(reserved_paths.insert(path_key(path)?))
}

/// 判断两个已存在路径是否指向同一个文件系统实体，用于跳过自替换和同目录移动。
pub(crate) fn paths_refer_to_same_entry(left: &Path, right: &Path) -> Result<bool, String> {
    if !left.exists() || !right.exists() {
        return Ok(false);
    }

    Ok(path_key(left)? == path_key(right)?)
}

/// 判断拖拽来源是否已经是目标目录的直接子项，移动时可提前跳过。
pub(crate) fn is_direct_child_of_destination(
    source: &Path,
    destination: &Path,
) -> Result<bool, String> {
    let Some(parent) = source.parent() else {
        return Ok(false);
    };

    Ok(path_key(parent)? == path_key(destination)?)
}

/// Windows 不允许把文件夹移动到自身内部；提前拦截比等待系统错误更清晰。
pub(crate) fn destination_is_inside_source(
    source: &Path,
    destination: &Path,
) -> Result<bool, String> {
    if !source.is_dir() {
        return Ok(false);
    }

    let source_key = path_key(source)?;
    let destination_key = path_key(destination)?;
    let separator = std::path::MAIN_SEPARATOR.to_string();
    let prefix = format!(
        "{}{}",
        source_key.trim_end_matches(std::path::MAIN_SEPARATOR),
        separator
    );

    Ok(destination_key.starts_with(&prefix))
}

fn split_duplicate_name(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "文件".to_string());
    let extension = path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();

    (stem, extension)
}

fn path_key(path: &Path) -> Result<String, String> {
    let absolute_path = if path.exists() {
        fs::canonicalize(path).map_err(|error| format!("无法规范化路径：{error}"))?
    } else if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("无法读取当前目录：{error}"))?
            .join(path)
    };
    let text = absolute_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        Ok(text.to_ascii_lowercase())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(text)
    }
}
