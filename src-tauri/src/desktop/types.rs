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
