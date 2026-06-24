//! 文件系统契约常量集中表达跨模块共享的文件类型约定。

/// Windows 快捷方式扩展名，Box 映射模式和 Shell 图标解析都依赖该类型判断。
pub const WINDOWS_SHORTCUT_EXTENSION: &str = "lnk";

/// Windows 快捷方式文件名后缀，生成映射文件时避免重复拼写 `.lnk`。
pub const WINDOWS_SHORTCUT_SUFFIX: &str = ".lnk";
