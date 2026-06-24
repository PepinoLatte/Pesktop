//! Box 文件操作策略类型集中在领域层，保证命令层和服务层解析同一份前端契约。

use std::str::FromStr;

use crate::domain::desktop::contract::{
    box_conflict_policy_code, box_delete_policy_code, box_drop_action_code,
};

/// 删除文件夹型 Box 时的真实文件处理策略，值与前端 `BoxDeletePolicy` 保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDeletePolicy {
    MoveContentsToDesktop,
    KeepFolder,
    RecycleFolder,
}

/// 文件拖入或拖出 Box 后的真实处理策略，值与前端 `BoxDropAction` 保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDropAction {
    Copy,
    Move,
    Map,
}

/// 目标目录存在同名文件时的处理策略，默认重命名以避免弹出 Windows 冲突框。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxConflictPolicy {
    Rename,
    Skip,
    Replace,
}

impl BoxDeletePolicy {
    /// 返回前端和数据库共同使用的策略代码，避免调用方反向硬编码协议字符串。
    pub const fn code(self) -> &'static str {
        match self {
            Self::MoveContentsToDesktop => box_delete_policy_code::MOVE_CONTENTS_TO_DESKTOP,
            Self::KeepFolder => box_delete_policy_code::KEEP_FOLDER,
            Self::RecycleFolder => box_delete_policy_code::RECYCLE_FOLDER,
        }
    }
}

impl BoxDropAction {
    /// 返回前端和数据库共同使用的拖拽操作代码，保持真实文件操作语义可追踪。
    pub const fn code(self) -> &'static str {
        match self {
            Self::Copy => box_drop_action_code::COPY,
            Self::Move => box_drop_action_code::MOVE,
            Self::Map => box_drop_action_code::MAP,
        }
    }
}

impl BoxConflictPolicy {
    /// 返回前端和数据库共同使用的同名处理代码，避免替换类危险操作出现分叉。
    pub const fn code(self) -> &'static str {
        match self {
            Self::Rename => box_conflict_policy_code::RENAME,
            Self::Skip => box_conflict_policy_code::SKIP,
            Self::Replace => box_conflict_policy_code::REPLACE,
        }
    }
}

impl FromStr for BoxDeletePolicy {
    type Err = String;

    /// 从前端设置字符串解析删除策略，避免危险文件操作接受任意未知值。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            box_delete_policy_code::MOVE_CONTENTS_TO_DESKTOP => Ok(Self::MoveContentsToDesktop),
            box_delete_policy_code::KEEP_FOLDER => Ok(Self::KeepFolder),
            box_delete_policy_code::RECYCLE_FOLDER => Ok(Self::RecycleFolder),
            _ => Err("未知的 Box 删除策略".to_string()),
        }
    }
}

impl FromStr for BoxDropAction {
    type Err = String;

    /// 从前端设置字符串解析拖入拖出策略，避免未知值触发真实文件操作。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            box_drop_action_code::COPY => Ok(Self::Copy),
            box_drop_action_code::MOVE => Ok(Self::Move),
            box_drop_action_code::MAP => Ok(Self::Map),
            _ => Err("未知的 Box 拖入处理方式".to_string()),
        }
    }
}

impl FromStr for BoxConflictPolicy {
    type Err = String;

    /// 从前端设置字符串解析同名处理策略，避免危险替换逻辑被任意字符串触发。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            box_conflict_policy_code::RENAME => Ok(Self::Rename),
            box_conflict_policy_code::SKIP => Ok(Self::Skip),
            box_conflict_policy_code::REPLACE => Ok(Self::Replace),
            _ => Err("未知的同名文件处理方式".to_string()),
        }
    }
}
