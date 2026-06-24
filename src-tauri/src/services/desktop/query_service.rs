//! 桌面文件查询服务负责桌面路径快照、Box 文件夹扫描和 Shell 虚拟项列表。

use crate::domain::desktop::{DesktopItem, DesktopSnapshot};
use crate::infrastructure::windows::shell_virtual_item;

use super::scan_service;

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
pub fn get_desktop_snapshot() -> DesktopSnapshot {
    scan_service::get_desktop_snapshot()
}

/// 扫描 Box 真实收纳文件夹的直接子项，供 WebView 自绘文件网格使用。
pub fn list_box_folder_items(folder_path: &str) -> Result<Vec<DesktopItem>, String> {
    scan_service::list_box_folder_items(folder_path)
}

/// 计算 Box 文件夹轻量版本号，前端轮询用它判断是否需要拉取完整图标列表。
pub fn get_box_folder_revision(folder_path: &str) -> Result<String, String> {
    scan_service::get_box_folder_revision(folder_path)
}

/// 根据 Box 持久化的 Shell ID 重建系统桌面图标展示项，未知项会被过滤掉。
pub fn list_shell_desktop_items(shell_ids: &[String]) -> Vec<DesktopItem> {
    shell_virtual_item::list_shell_virtual_desktop_items(shell_ids)
}
