//! 应用设置契约常量；当前设置由前端 SQLite 插件写入，Rust 侧保留同名契约避免后续重复硬编码。

/// 应用设置表名，必须与前端 `APP_SETTINGS_STORAGE.tables.appSettings` 保持一致。
pub const APP_SETTINGS_TABLE: &str = "app_settings";

/// 应用设置键名，必须与前端 `APP_SETTING_KEYS` 保持一致。
pub mod keys {
    /// Box 内项目是否需要双击打开。
    pub const DOUBLE_CLICK_OPEN_ITEMS: &str = "doubleClickOpenItems";
    /// Box 内文件名后缀显示策略。
    pub const NAME_DISPLAY_MODE: &str = "nameDisplayMode";
    /// Box 内是否显示项目名称。
    pub const SHOW_ITEM_LABELS: &str = "showItemLabels";
    /// Box 内是否显示快捷方式箭头标记。
    pub const SHOW_SHORTCUT_ARROW: &str = "showShortcutArrow";
    /// Box 吸附阈值。
    pub const SNAP_THRESHOLD: &str = "snapThreshold";
    /// Box 是否吸附屏幕或其他 Box 边缘。
    pub const SNAP_TO_EDGES: &str = "snapToEdges";
    /// 设置页和 Box 的主题模式。
    pub const THEME: &str = "theme";
}
