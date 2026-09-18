//! 文件传输执行器负责按计划调用 Shell、复制文件并在失败时回滚。

use std::path::{Path, PathBuf};

use crate::infrastructure::filesystem::transfer::backup;
use crate::infrastructure::filesystem::transfer::copy::copy_path_without_shell_prompt;
use crate::infrastructure::filesystem::transfer::plan::{
    CompletedTransfer, FileTransferKind, FileTransferPlan,
};
use crate::infrastructure::filesystem::transfer::rollback;
use crate::infrastructure::windows::shell_file;

/// 执行完整传输批次，返回实际落地的目标路径，供前端继续写入桌面排序。
pub(super) fn execute_transfer_plans(plans: &[FileTransferPlan]) -> Result<Vec<PathBuf>, String> {
    let mut completed_transfers = Vec::new();
    let mut index = 0;

    while index < plans.len() {
        let plan = &plans[index];
        if plan.can_batch_shell_move() {
            let mut next_index = index + 1;
            while next_index < plans.len() && plans[next_index].can_batch_shell_move() {
                next_index += 1;
            }

            if let Err(error) = execute_shell_move_batch(&plans[index..next_index]) {
                rollback::rollback_failed_move_batch(&plans[index..next_index]);
                rollback::rollback_completed_transfers(&completed_transfers);
                return Err(error);
            }

            completed_transfers.extend(
                plans[index..next_index]
                    .iter()
                    .map(FileTransferPlan::completed_without_backup),
            );
            index = next_index;
            continue;
        }

        // 替换策略先把旧目标改名成隐藏备份，只有整批传输成功后才清理，保证失败可回滚。
        let backup_path = if plan.replaces_existing {
            backup::prepare_replace_backup(&plan.destination)?
        } else {
            None
        };

        if let Err(error) = execute_single_transfer(plan) {
            rollback::cleanup_failed_single_transfer(plan);
            if let Some(existing_backup) = backup_path {
                let _ = backup::restore_replace_backup(&existing_backup, &plan.destination);
            }
            rollback::rollback_completed_transfers(&completed_transfers);
            return Err(error);
        }

        completed_transfers.push(CompletedTransfer {
            source: plan.source.clone(),
            destination: plan.destination.clone(),
            kind: plan.kind,
            backup: backup_path,
        });
        index += 1;
    }

    backup::cleanup_replace_backups(&completed_transfers)?;

    Ok(completed_transfers
        .iter()
        .map(|transfer| transfer.destination.clone())
        .collect())
}

/// 移动单个路径；内部使用 Shell 文件操作，让 Explorer 立即收到桌面文件增删事件。
pub(super) fn move_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("目标文件已经存在，已停止移动".to_string());
    }

    shell_file::move_paths_with_shell(&[(source.to_path_buf(), target.to_path_buf())])
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

/// 执行计划时连续的移动操作可以合并为一次 Shell 调用，避免多文件拖拽反复刷新桌面。
fn execute_shell_move_batch(plans: &[FileTransferPlan]) -> Result<(), String> {
    let moves = plans
        .iter()
        .map(|plan| (plan.source.clone(), plan.destination.clone()))
        .collect::<Vec<_>>();

    shell_file::move_paths_with_shell(&moves)
}
