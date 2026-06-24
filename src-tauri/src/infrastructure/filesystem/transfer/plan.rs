//! 文件传输计划和执行结果的内部模型。

use std::path::PathBuf;

/// 单个文件项的传输方式，统一表达复制、移动和映射快捷方式三类业务动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileTransferKind {
    Copy,
    Move,
    Shortcut,
}

/// 传输前解析出的确定性计划，执行层只关注计划，不再重复判断拖拽策略。
#[derive(Debug, Clone)]
pub(super) struct FileTransferPlan {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
    pub(super) kind: FileTransferKind,
    pub(super) replaces_existing: bool,
}

impl FileTransferPlan {
    /// Shell 批量移动只适用于纯移动且不需要替换备份的计划，替换场景必须单独执行以保证可回滚。
    pub(super) fn can_batch_shell_move(&self) -> bool {
        self.kind == FileTransferKind::Move && !self.replaces_existing
    }

    /// 把已成功执行的计划转换为回滚记录；批量移动没有替换备份，统一在这里生成记录。
    pub(super) fn completed_without_backup(&self) -> CompletedTransfer {
        CompletedTransfer {
            source: self.source.clone(),
            destination: self.destination.clone(),
            kind: self.kind,
            backup: None,
        }
    }
}

/// 已完成的传输记录用于失败回滚；替换策略需要额外记录旧目标备份路径。
#[derive(Debug, Clone)]
pub(super) struct CompletedTransfer {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
    pub(super) kind: FileTransferKind,
    pub(super) backup: Option<PathBuf>,
}
