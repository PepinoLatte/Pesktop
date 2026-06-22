//! Windows 平台能力统一收口，非 Windows 兜底实现也放在同名模块内保持调用方稳定。

pub(crate) mod desktop_path;
pub(crate) mod folder_dialog;
pub(crate) mod mouse;
pub(crate) mod native_drop;
pub(crate) mod shell_context;
pub(crate) mod shell_file;
pub(crate) mod shell_icon;
