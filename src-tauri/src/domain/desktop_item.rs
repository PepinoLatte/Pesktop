//! Box 文件网格的领域模型只表达前端展示和后续文件操作所需的稳定字段。

use serde::Serialize;

/// 文件剪贴板操作只接受复制和剪切两种意图，对应 Windows Shell 的 Copy/Move DropEffect。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClipboardOperation {
    Copy,
    Cut,
}

impl FileClipboardOperation {
    /// 从前端快捷键命令解析剪贴板意图，避免未知字符串进入真实文件操作链路。
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "copy" => Ok(Self::Copy),
            "cut" => Ok(Self::Cut),
            _ => Err("未知的文件剪贴板操作".to_string()),
        }
    }
}

/// 桌面路径快照只服务删除策略，文件展示由 Box 文件夹扫描命令独立提供。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    pub desktop_path: String,
}

/// Box 文件项类型只表达前端展示所需语义，真实打开方式仍交给 Windows Shell 判断。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopItemKind {
    File,
    Folder,
    Shortcut,
    Unknown,
}

/// Box 文件项是真实文件夹的直接子项快照，`path` 是后续文件操作的唯一稳定主键。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopItem {
    pub extension: Option<String>,
    pub icon_data_url: Option<String>,
    pub id: String,
    pub kind: DesktopItemKind,
    pub name: String,
    pub path: String,
}
