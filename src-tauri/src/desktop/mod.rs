//! 桌面模块提供真实文件夹 Box 所需的文件系统和 Shell 能力，文件区由 WebView 单树渲染。

mod folder_ops;
mod native_drop;
mod scanner;
mod types;

pub use folder_ops::{
    choose_collection_root_folder, create_box_folder, delete_box_folder,
    handle_box_dragged_paths_to_desktop, handle_box_dropped_paths, migrate_box_folder,
    open_folder_in_explorer, open_item_with_system_default, recycle_items, rename_item,
    BoxConflictPolicy, BoxDeletePolicy, BoxDropAction,
};
pub use native_drop::{register_box_native_drop, unregister_box_native_drop};
pub use scanner::{scan_box_folder, scan_desktop};
pub use types::{DesktopItem, DesktopItemKind, DesktopSnapshot};
