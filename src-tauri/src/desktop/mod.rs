//! 桌面扫描模块只读取真实桌面文件元信息，不接管 Windows Shell 渲染层

mod native_drop;
mod native_icons;
mod scanner;
mod types;

pub use native_drop::{register_box_native_drop, unregister_box_native_drop};
pub use native_icons::set_native_desktop_icons_hidden;
pub use scanner::{
    resolve_shell_item_path_from_parsing_name, resolve_shell_parsing_name, scan_desktop, scan_paths,
};
pub use types::{DesktopItem, DesktopItemKind, DesktopItemSource, DesktopSnapshot};
