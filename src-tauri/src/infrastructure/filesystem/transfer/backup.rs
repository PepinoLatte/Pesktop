//! 替换冲突策略的备份与恢复逻辑。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::infrastructure::filesystem::transfer::copy::remove_path_if_exists;
use crate::infrastructure::filesystem::transfer::plan::CompletedTransfer;

/// 替换备份路径最多尝试次数，避免极端同名隐藏文件导致无界循环。
const REPLACE_BACKUP_CANDIDATE_LIMIT: usize = 10_000;
/// 替换备份文件名前缀，用命名常量表达 Dasktop 内部临时文件契约。
const REPLACE_BACKUP_FILE_PREFIX: &str = ".dasktop_replace_backup";
/// 无法从目标路径得到文件名时使用的兜底名称，避免生成空文件名。
const FALLBACK_REPLACE_BACKUP_FILE_NAME: &str = "item";

/// 若目标已存在，先把旧目标移动到隐藏备份路径，整批成功后再统一清理。
pub(super) fn prepare_replace_backup(destination: &Path) -> Result<Option<PathBuf>, String> {
    if !destination.exists() {
        return Ok(None);
    }

    let backup_path = resolve_replace_backup_path(destination)?;
    fs::rename(destination, &backup_path)
        .map_err(|error| format!("无法备份同名目标，已停止替换：{error}"))?;

    Ok(Some(backup_path))
}

/// 传输失败时恢复被替换的旧目标，恢复前会清理已写入但不完整的新目标。
pub(super) fn restore_replace_backup(backup: &Path, destination: &Path) -> Result<(), String> {
    let _ = remove_path_if_exists(destination);
    fs::rename(backup, destination).map_err(|error| format!("无法恢复被替换的同名文件：{error}"))
}

/// 整批传输成功后删除所有旧目标备份，替换策略在这里正式提交。
pub(super) fn cleanup_replace_backups(
    completed_transfers: &[CompletedTransfer],
) -> Result<(), String> {
    for transfer in completed_transfers {
        if let Some(backup) = &transfer.backup {
            remove_path_if_exists(backup)?;
        }
    }

    Ok(())
}

fn resolve_replace_backup_path(destination: &Path) -> Result<PathBuf, String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "无法解析同名目标所在文件夹".to_string())?;
    let file_name = destination
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| FALLBACK_REPLACE_BACKUP_FILE_NAME.to_string());
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    for index in 0..REPLACE_BACKUP_CANDIDATE_LIMIT {
        let candidate = parent.join(format!(
            "{REPLACE_BACKUP_FILE_PREFIX}_{stamp}_{index}_{file_name}"
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("无法创建同名替换备份路径".to_string())
}
