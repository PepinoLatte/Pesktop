//! 文件传输失败回滚逻辑，集中处理移动、复制、快捷方式和替换备份恢复。

use crate::infrastructure::filesystem::transfer::backup;
use crate::infrastructure::filesystem::transfer::copy::remove_path_if_exists;
use crate::infrastructure::filesystem::transfer::executor::move_path_without_shell_prompt;
use crate::infrastructure::filesystem::transfer::plan::{
    CompletedTransfer, FileTransferKind, FileTransferPlan,
};

/// 清理单个传输失败后可能留下的目标路径，移动失败则尝试按 Shell 实际状态回滚。
pub(super) fn cleanup_failed_single_transfer(plan: &FileTransferPlan) {
    match plan.kind {
        FileTransferKind::Move => rollback_failed_move_batch(std::slice::from_ref(plan)),
        FileTransferKind::Copy | FileTransferKind::Shortcut => {
            let _ = remove_path_if_exists(&plan.destination);
        }
    }
}

/// Shell 文件操作可能在批量移动中途返回失败；只有确认来源已消失且目标存在时才回滚，避免误删用户数据。
pub(super) fn rollback_failed_move_batch(plans: &[FileTransferPlan]) {
    for plan in plans.iter().rev() {
        if !plan.source.exists() && plan.destination.exists() {
            let _ = move_path_without_shell_prompt(&plan.destination, &plan.source);
        }
    }
}

/// 回滚已完成传输；按完成顺序倒序执行，避免目录移动时父子路径互相占用。
pub(super) fn rollback_completed_transfers(completed_transfers: &[CompletedTransfer]) {
    for transfer in completed_transfers.iter().rev() {
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

        if let Some(existing_backup) = &transfer.backup {
            let _ = backup::restore_replace_backup(existing_backup, &transfer.destination);
        }
    }
}
