//! 桌面扫描模块只读取真实桌面文件元信息，不接管 Windows Shell 渲染层。

mod native_icons;
mod scanner;
mod types;

pub use native_icons::set_native_desktop_icons_hidden;
pub use scanner::{scan_desktop, scan_paths};
pub use types::{DesktopItem, DesktopItemKind, DesktopSnapshot};
