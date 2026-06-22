//! 应用设置契约常量；当前设置由前端 SQLite 插件写入，Rust 侧保留同名契约避免后续重复硬编码

/// 应用设置表名，必须与前端 `APP_SETTINGS_STORAGE.tables.appSettings` 保持一致
pub const APP_SETTINGS_TABLE: &str = "app_settings";

/// 应用设置键名，必须与前端 `APP_SETTING_KEYS` 保持一致
pub mod keys {
    /// Box 背景透明度百分比
    pub const BOX_BACKGROUND_OPACITY: &str = "boxBackgroundOpacity";
    /// Box 收缩和展开动画持续时间
    pub const BOX_COLLAPSE_ANIMATION_MS: &str = "boxCollapseAnimationMs";
    /// Box 整体和图标命中区域圆角
    pub const BOX_CORNER_RADIUS: &str = "boxCornerRadius";
    /// Box 手动 resize 是否按图标网格吸附
    pub const BOX_RESIZE_GRID_ENABLED: &str = "boxResizeGridEnabled";
    /// Box 传输遇到同名文件时的处理策略，必须与前端 `BoxConflictPolicy` 保持同名语义
    pub const BOX_CONFLICT_POLICY: &str = "boxConflictPolicy";
    /// 删除 Box 时真实文件夹的处理策略，必须与前端 `BoxDeletePolicy` 保持同名语义
    pub const BOX_DELETE_POLICY: &str = "boxDeletePolicy";
    /// Box 文件拖出到桌面后的处理策略，必须与前端 `BoxDropAction` 保持同名语义
    pub const BOX_DRAG_OUT_ACTION: &str = "boxDragOutAction";
    /// 外部文件拖入 Box 后的处理策略，必须与前端 `BoxDropAction` 保持同名语义
    pub const BOX_DROP_ACTION: &str = "boxDropAction";
    /// Box 窗口主题模式
    pub const BOX_THEME: &str = "boxTheme";
    /// 新建 Box 的真实文件夹根目录；修改后只影响后续 Box，历史 Box 使用自身 `folderPath`
    pub const COLLECTION_ROOT_PATH: &str = "collectionRootPath";
    /// 设置页主题模式
    pub const SETTINGS_THEME: &str = "settingsTheme";
    /// Box 吸附阈值
    pub const SNAP_THRESHOLD: &str = "snapThreshold";
    /// Box 是否吸附屏幕或其他 Box 边缘
    pub const SNAP_TO_EDGES: &str = "snapToEdges";
}
