//! 桌面扫描模块只读取真实桌面文件元信息，不接管 Windows Shell 渲染层。

mod scanner;
mod types;

pub use scanner::scan_desktop;
pub use types::{DesktopItem, DesktopItemKind, DesktopSnapshot};
