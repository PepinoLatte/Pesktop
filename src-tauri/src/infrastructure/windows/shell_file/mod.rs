//! Windows Shell 文件能力封装，集中处理系统默认打开、回收站和快捷方式创建。

mod execute;
mod operation;
mod shortcut;

pub use execute::{
    open_folder_in_explorer, open_item_with_system_default, open_parsing_name_with_system_default,
};
pub use operation::{recycle_path, recycle_paths};
pub use shortcut::create_shortcut_for_path;

pub(crate) use operation::move_paths_with_shell;
