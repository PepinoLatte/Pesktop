//! 文件传输基础设施统一处理复制、移动、映射、同名冲突和失败回滚。

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::box_policy::{BoxConflictPolicy, BoxDropAction};
use crate::infrastructure::filesystem::naming;
use crate::infrastructure::windows::shell_file;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileTransferKind {
    Copy,
    Move,
    Shortcut,
}

#[derive(Debug, Clone)]
struct FileTransferPlan {
    source: PathBuf,
    destination: PathBuf,
    kind: FileTransferKind,
    replaces_existing: bool,
}

#[derive(Debug, Clone)]
struct CompletedTransfer {
    source: PathBuf,
    destination: PathBuf,
    kind: FileTransferKind,
    backup: Option<PathBuf>,
}

/// 过滤并规范化外部传入的真实路径，任何不存在的路径都会阻止本批次继续执行。
pub(crate) fn normalize_existing_paths(paths: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut normalized_paths = Vec::new();

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        if !path.exists() {
            return Err(format!("拖入项目不存在：{}", path.to_string_lossy()));
        }
        normalized_paths.push(path);
    }

    Ok(normalized_paths)
}

/// 普通拖拽传输统一绕开 Shell 文件操作，避免冲突框和权限提升框被 Box 窗口遮挡。
/// 返回实际完成的目标路径，供前端按释放位置写入手动排序；冲突跳过的项目不会出现在结果中。
pub(crate) fn transfer_paths_without_shell_prompts(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<PathBuf>, String> {
    let plans = create_transfer_plans(paths, destination, action, conflict_policy)?;
    execute_transfer_plans(&plans)
}

/// 移动单个路径；跨盘时退回复制后删除，保持迁移 Box 文件夹时的用户预期。
pub(crate) fn move_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("目标文件已经存在，已停止移动".to_string());
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if is_cross_device_move_error(&error) => {
            if let Err(copy_error) = copy_path_without_shell_prompt(source, target) {
                let _ = remove_path_if_exists(target);
                return Err(format!("无法跨盘移动文件项：{copy_error}"));
            }
            if let Err(remove_error) = remove_original_after_copy(source) {
                let _ = remove_path_if_exists(target);
                return Err(format!("无法删除原文件，已取消跨盘移动：{remove_error}"));
            }

            Ok(())
        }
        Err(error) => Err(format!(
            "无法移动文件项，可能没有写入权限或文件正在使用：{error}"
        )),
    }
}

fn create_transfer_plans(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<FileTransferPlan>, String> {
    let mut reserved_paths = HashSet::new();
    let mut plans = Vec::new();

    for source in paths {
        // 同目录移动没有真实文件变化，提前跳过可以避免后续把自身当成冲突目标处理。
        if action == BoxDropAction::Move
            && naming::is_direct_child_of_destination(source, destination)?
        {
            continue;
        }
        // Windows 不允许把文件夹移动到自身内部；提前拦截比等待 fs::rename 报模糊错误更清晰。
        if action == BoxDropAction::Move
            && naming::destination_is_inside_source(source, destination)?
        {
            return Err("不能把文件夹移动到自身内部".to_string());
        }

        let (kind, desired_destination) =
            resolve_desired_transfer_destination(source, destination, action)?;
        let Some(destination_path) = resolve_conflict_destination(
            source,
            &desired_destination,
            conflict_policy,
            &mut reserved_paths,
        )?
        else {
            continue;
        };

        plans.push(FileTransferPlan {
            source: source.clone(),
            replaces_existing: conflict_policy == BoxConflictPolicy::Replace
                && destination_path.exists(),
            destination: destination_path,
            kind,
        });
    }

    Ok(plans)
}

fn resolve_desired_transfer_destination(
    source: &Path,
    destination: &Path,
    action: BoxDropAction,
) -> Result<(FileTransferKind, PathBuf), String> {
    if action == BoxDropAction::Map && !is_windows_shortcut(source) {
        return Ok((
            FileTransferKind::Shortcut,
            naming::desired_shortcut_path(source, destination),
        ));
    }

    let file_name = source
        .file_name()
        .ok_or_else(|| "无法解析待处理项目名称".to_string())?;
    let kind = match action {
        BoxDropAction::Copy | BoxDropAction::Map => FileTransferKind::Copy,
        BoxDropAction::Move => FileTransferKind::Move,
    };

    Ok((kind, destination.join(file_name)))
}

fn resolve_conflict_destination(
    source: &Path,
    desired_destination: &Path,
    conflict_policy: BoxConflictPolicy,
    reserved_paths: &mut HashSet<String>,
) -> Result<Option<PathBuf>, String> {
    if conflict_policy == BoxConflictPolicy::Replace
        && naming::paths_refer_to_same_entry(source, desired_destination)?
    {
        return Ok(None);
    }

    match conflict_policy {
        BoxConflictPolicy::Rename => {
            naming::resolve_renamed_destination(desired_destination, reserved_paths).map(Some)
        }
        BoxConflictPolicy::Skip => {
            if desired_destination.exists()
                || !naming::reserve_path(desired_destination, reserved_paths)?
            {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
        BoxConflictPolicy::Replace => {
            // 同一批次内两个来源解析到同一路径时跳过后者，避免“替换”把前一个刚传输的文件覆盖掉。
            if !naming::reserve_path(desired_destination, reserved_paths)? {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
    }
}

fn execute_transfer_plans(plans: &[FileTransferPlan]) -> Result<Vec<PathBuf>, String> {
    let mut completed_transfers = Vec::new();

    for plan in plans {
        // 替换策略先把旧目标改名成隐藏备份，只有整批传输成功后才清理，保证失败可回滚。
        let backup = if plan.replaces_existing {
            prepare_replace_backup(&plan.destination)?
        } else {
            None
        };

        if let Err(error) = execute_single_transfer(plan) {
            let _ = remove_path_if_exists(&plan.destination);
            if let Some(backup_path) = backup {
                let _ = restore_replace_backup(&backup_path, &plan.destination);
            }
            rollback_completed_transfers(&completed_transfers);
            return Err(error);
        }

        completed_transfers.push(CompletedTransfer {
            source: plan.source.clone(),
            destination: plan.destination.clone(),
            kind: plan.kind,
            backup,
        });
    }

    cleanup_replace_backups(&completed_transfers)?;

    Ok(completed_transfers
        .iter()
        .map(|transfer| transfer.destination.clone())
        .collect())
}

fn execute_single_transfer(plan: &FileTransferPlan) -> Result<(), String> {
    match plan.kind {
        FileTransferKind::Copy => copy_path_without_shell_prompt(&plan.source, &plan.destination),
        FileTransferKind::Move => move_path_without_shell_prompt(&plan.source, &plan.destination),
        FileTransferKind::Shortcut => {
            shell_file::create_shortcut_for_path(&plan.source, &plan.destination)
        }
    }
}

fn rollback_completed_transfers(completed_transfers: &[CompletedTransfer]) {
    for transfer in completed_transfers.iter().rev() {
        // 回滚按完成顺序倒序执行，避免目录移动时父子路径互相占用。
        match transfer.kind {
            FileTransferKind::Move => {
                if transfer.destination.exists() {
                    let _ = move_path_without_shell_prompt(&transfer.destination, &transfer.source);
                }
            }
            FileTransferKind::Copy | FileTransferKind::Shortcut => {
                let _ = remove_path_if_exists(&transfer.destination);
            }
        }

        if let Some(backup) = &transfer.backup {
            let _ = restore_replace_backup(backup, &transfer.destination);
        }
    }
}

fn cleanup_replace_backups(completed_transfers: &[CompletedTransfer]) -> Result<(), String> {
    for transfer in completed_transfers {
        if let Some(backup) = &transfer.backup {
            remove_path_if_exists(backup)?;
        }
    }

    Ok(())
}

fn prepare_replace_backup(destination: &Path) -> Result<Option<PathBuf>, String> {
    if !destination.exists() {
        return Ok(None);
    }

    let backup_path = resolve_replace_backup_path(destination)?;
    fs::rename(destination, &backup_path)
        .map_err(|error| format!("无法备份同名目标，已停止替换：{error}"))?;

    Ok(Some(backup_path))
}

fn restore_replace_backup(backup: &Path, destination: &Path) -> Result<(), String> {
    let _ = remove_path_if_exists(destination);
    fs::rename(backup, destination).map_err(|error| format!("无法恢复被替换的同名文件：{error}"))
}

fn resolve_replace_backup_path(destination: &Path) -> Result<PathBuf, String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "无法解析同名目标所在文件夹".to_string())?;
    let file_name = destination
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "item".to_string());
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    for index in 0..10_000 {
        let candidate = parent.join(format!(
            ".dasktop_replace_backup_{}_{}_{}",
            stamp, index, file_name
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("无法创建同名替换备份路径".to_string())
}

fn copy_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("目标文件已经存在，已停止复制".to_string());
    }

    if source.is_dir() {
        copy_directory_without_shell_prompt(source, target)
    } else if source.is_file() {
        fs::copy(source, target)
            .map(|_| ())
            .map_err(|error| format!("无法复制文件项：{error}"))
    } else {
        Err(format!(
            "无法处理未知类型的文件项：{}",
            source.to_string_lossy()
        ))
    }
}

fn copy_directory_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir(target).map_err(|error| format!("无法创建目标文件夹：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("无法读取源文件夹：{error}"))?
    {
        let entry = entry.map_err(|error| format!("无法读取源文件夹内容：{error}"))?;
        let source_child = entry.path();
        let target_child = target.join(entry.file_name());

        if let Err(error) = copy_path_without_shell_prompt(&source_child, &target_child) {
            let _ = remove_path_if_exists(target);
            return Err(error);
        }
    }

    Ok(())
}

fn remove_original_after_copy(source: &Path) -> Result<(), String> {
    if source.is_dir() {
        fs::remove_dir_all(source).map_err(|error| format!("无法删除原文件夹：{error}"))
    } else {
        fs::remove_file(source).map_err(|error| format!("无法删除原文件：{error}"))
    }
}

fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("无法清理目标文件夹：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("无法清理目标文件：{error}"))
    }
}

/// Windows 跨盘重命名会返回 ERROR_NOT_SAME_DEVICE；这时可以退回复制后删除。
fn is_cross_device_move_error(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(17))
}

fn is_windows_shortcut(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
}
