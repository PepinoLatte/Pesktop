//! Box 文件夹服务编排收纳目录、拖入拖出、原生拖放注册等业务用例。

pub mod drop_service;
pub mod folder_service;

pub use drop_service::{
    handle_box_dragged_paths_to_desktop, handle_box_dropped_paths, register_box_native_drop,
    unregister_box_native_drop,
};
pub use folder_service::{
    choose_collection_root_folder, create_box_folder, delete_box_folder, migrate_box_folder,
    open_box_folder,
};
