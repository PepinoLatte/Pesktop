//! 桌面和 Box 文件操作的前端协议值，集中表达危险文件操作允许的白名单。

/// 前端传入的删除策略代码值集中维护，避免领域枚举解析重复硬编码。
pub mod box_delete_policy_code {
    /// 删除 Box 时先把内容移回桌面。
    pub const MOVE_CONTENTS_TO_DESKTOP: &str = "moveContentsToDesktop";
    /// 删除 Box 记录但保留真实文件夹。
    pub const KEEP_FOLDER: &str = "keepFolder";
    /// 删除 Box 时将真实文件夹移入回收站。
    pub const RECYCLE_FOLDER: &str = "recycleFolder";
}

/// 前端传入的拖拽处理代码值集中维护，避免领域枚举解析重复硬编码。
pub mod box_drop_action_code {
    /// 拖拽后复制源文件。
    pub const COPY: &str = "copy";
    /// 拖拽后移动源文件。
    pub const MOVE: &str = "move";
    /// 拖拽后创建 Windows 快捷方式映射。
    pub const MAP: &str = "map";
}

/// 前端传入的同名冲突策略代码值集中维护，避免危险替换逻辑接收任意字符串。
pub mod box_conflict_policy_code {
    /// 冲突时自动生成副本名称。
    pub const RENAME: &str = "rename";
    /// 冲突时跳过当前文件。
    pub const SKIP: &str = "skip";
    /// 冲突时替换目标文件。
    pub const REPLACE: &str = "replace";
}

/// 前端传入的剪贴板操作代码值集中维护，避免复制/剪切语义出现分叉。
pub mod file_clipboard_operation_code {
    /// 文件复制意图。
    pub const COPY: &str = "copy";
    /// 文件剪切意图。
    pub const CUT: &str = "cut";
}
