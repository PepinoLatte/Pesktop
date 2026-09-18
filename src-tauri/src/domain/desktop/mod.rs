//! 桌面与 Box 文件网格领域模型，统一收口策略、展示模型和剪贴板操作。

pub mod clipboard;
pub mod contract;
pub mod item;
pub mod policy;

pub use clipboard::FileClipboardOperation;
pub use item::{DesktopItem, DesktopItemKind, DesktopItemSource, DesktopSnapshot};
pub use policy::{BoxConflictPolicy, BoxDeletePolicy, BoxDropAction};
