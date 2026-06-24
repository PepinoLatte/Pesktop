//! Box 文件网格的领域模型只表达前端展示和后续文件操作所需的稳定字段。

use serde::Serialize;

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
    Shell,
    Shortcut,
    Unknown,
}

/// Box 文件项来源决定后续命令把 `path` 当作真实文件路径还是 Shell 虚拟项稳定键。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopItemSource {
    FileSystem,
    Shell,
}

/// Box 文件项是真实文件或 Shell 虚拟项的展示快照，`path` 是前端列表和排序的稳定主键。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopItem {
    pub extension: Option<String>,
    pub icon_data_url: Option<String>,
    pub id: String,
    pub kind: DesktopItemKind,
    pub name: String,
    pub path: String,
    pub shell_id: Option<String>,
    pub source: DesktopItemSource,
}
