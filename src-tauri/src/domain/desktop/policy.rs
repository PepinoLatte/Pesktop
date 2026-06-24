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

#[cfg(test)]
mod tests {
    use super::*;

    /// 前端策略字符串是危险文件操作的入口，标准 FromStr 必须只接受明确白名单。
    #[test]
    fn parses_known_box_policies() {
        assert_eq!(
            "moveContentsToDesktop".parse::<BoxDeletePolicy>(),
            Ok(BoxDeletePolicy::MoveContentsToDesktop)
        );
        assert_eq!("map".parse::<BoxDropAction>(), Ok(BoxDropAction::Map));
        assert_eq!(
            "replace".parse::<BoxConflictPolicy>(),
            Ok(BoxConflictPolicy::Replace)
        );
    }

    /// 未知策略必须失败，避免命令层把任意字符串静默映射成高风险默认行为。
    #[test]
    fn rejects_unknown_box_policies() {
        assert!("delete".parse::<BoxDeletePolicy>().is_err());
        assert!("link".parse::<BoxDropAction>().is_err());
        assert!("overwrite".parse::<BoxConflictPolicy>().is_err());
    }
}
