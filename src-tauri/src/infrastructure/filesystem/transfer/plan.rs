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

/// 已完成的传输记录用于失败回滚；替换策略需要额外记录旧目标备份路径。
#[derive(Debug, Clone)]
pub(super) struct CompletedTransfer {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
    pub(super) kind: FileTransferKind,
    pub(super) backup: Option<PathBuf>,
}
