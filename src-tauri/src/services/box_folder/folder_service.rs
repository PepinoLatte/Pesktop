//! Box 真实文件夹服务，负责创建、迁移、删除和打开收纳目录。

use crate::domain::desktop::{BoxConflictPolicy, BoxDeletePolicy};
use crate::infrastructure::filesystem::box_folder as box_folder_fs;
use crate::infrastructure::windows::folder_dialog::{self, FolderDialogOwner};
use crate::infrastructure::windows::shell_file;

/// 打开系统文件夹选择器，选择后续新建 Box 使用的收纳根目录。
pub fn choose_collection_root_folder(owner: FolderDialogOwner) -> Result<Option<String>, String> {
    folder_dialog::choose_collection_root_folder(owner)
}

/// 在用户设置的收纳根目录下创建单个 Box 的真实文件夹。
pub fn create_box_folder(root_path: &str, folder_name: &str) -> Result<String, String> {
    box_folder_fs::create_box_folder(root_path, folder_name)
}

/// 按当前删除策略处理 Box 真实文件夹；成功后前端才会删除 Box 记录。
pub fn delete_box_folder(
    folder_path: &str,
    desktop_path: &str,
    policy: BoxDeletePolicy,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    box_folder_fs::delete_box_folder(folder_path, desktop_path, policy, conflict_policy)
}

/// 把单个 Box 的真实文件夹移动到当前收纳根目录，成功后返回新的文件夹路径。
pub fn migrate_box_folder(folder_path: &str, root_path: &str) -> Result<String, String> {
    box_folder_fs::migrate_box_folder(folder_path, root_path)
}

/// 用系统 Explorer 打开 Box 对应的真实文件夹。
pub fn open_box_folder(folder_path: &str) -> Result<(), String> {
    shell_file::open_folder_in_explorer(folder_path)
}
