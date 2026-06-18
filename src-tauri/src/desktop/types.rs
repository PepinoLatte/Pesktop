use serde::Serialize;

/// 桌面文件类型只描述渲染语义，不改变真实文件行为
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopItemKind {
    File,
    Folder,
    Shell,
    Shortcut,
    Unknown,
}

/// 桌面项目来源决定后端使用文件系统路径还是 Shell PIDL 执行打开、右键和拖放解析
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopItemSource {
    FileSystem,
    Shell,
}

/// 自绘桌面图标模型，path 是分组映射的稳定主键
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: Option<String>,
    pub kind: DesktopItemKind,
    pub icon_data_url: Option<String>,
    pub source: DesktopItemSource,
    pub shell_id: Option<String>,
}

/// 桌面快照由真实桌面目录扫描得出，前端据此重绘 UI
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    pub desktop_path: String,
    pub items: Vec<DesktopItem>,
}
