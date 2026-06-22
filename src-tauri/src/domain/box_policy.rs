//! Box 文件操作策略类型集中在领域层，保证命令层和服务层解析同一份前端契约。

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
    /// 从前端设置字符串解析删除策略，避免危险文件操作接受任意未知值。
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "moveContentsToDesktop" => Ok(Self::MoveContentsToDesktop),
            "keepFolder" => Ok(Self::KeepFolder),
            "recycleFolder" => Ok(Self::RecycleFolder),
            _ => Err("未知的 Box 删除策略".to_string()),
        }
    }
}

impl BoxDropAction {
    /// 从前端设置字符串解析拖入拖出策略，避免未知值触发真实文件操作。
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "copy" => Ok(Self::Copy),
            "move" => Ok(Self::Move),
            "map" => Ok(Self::Map),
            _ => Err("未知的 Box 拖入处理方式".to_string()),
        }
    }
}

impl BoxConflictPolicy {
    /// 从前端设置字符串解析同名处理策略，避免危险替换逻辑被任意字符串触发。
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "rename" => Ok(Self::Rename),
            "skip" => Ok(Self::Skip),
            "replace" => Ok(Self::Replace),
            _ => Err("未知的同名文件处理方式".to_string()),
        }
    }
}
