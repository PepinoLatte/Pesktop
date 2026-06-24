//! Windows Shell 虚拟桌面项封装，负责把“此电脑”等无文件路径对象映射成稳定业务引用。

mod cache;
pub(crate) mod catalog;
mod identity;
mod user_folder;

use crate::domain::desktop::{DesktopItem, DesktopItemKind, DesktopItemSource};
use crate::infrastructure::windows::{shell_context, shell_file, shell_icon};

/// Box 持久化 Shell 虚拟项时使用的路径前缀，前端排序和选择逻辑可继续复用 path 主键。
pub const SHELL_ITEM_PATH_PREFIX: &str = "shell::";

/// 根据持久化的 Shell ID 重建前端文件项模型，未知 ID 会被静默丢弃。
pub fn list_shell_virtual_desktop_items(shell_ids: &[String]) -> Vec<DesktopItem> {
    shell_ids
        .iter()
        .filter_map(|shell_id| cache::resolve_cached_shell_desktop_item(shell_id))
        .collect()
}

/// 判断路径键是否为 Dasktop 自己生成的 Shell 虚拟项引用。
pub fn strip_shell_item_path(path: &str) -> Option<&str> {
    path.strip_prefix(SHELL_ITEM_PATH_PREFIX)
}

/// 打开 Shell 虚拟项时使用 Windows Shell 解析名，而不是要求存在真实文件路径。
pub fn open_shell_virtual_item(shell_id: &str) -> Result<(), String> {
    let parsing_name = catalog::resolve_shell_parsing_name(shell_id)
        .ok_or_else(|| "暂不支持打开该系统桌面项目".to_string())?;

    shell_file::open_parsing_name_with_system_default(&parsing_name)
}

/// 在 Shell 虚拟项上弹出 Explorer 原生菜单，菜单命令仍由系统根据 PIDL 执行。
pub fn show_shell_virtual_item_context_menu(
    window: &tauri::WebviewWindow,
    shell_id: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let parsing_name = catalog::resolve_shell_parsing_name(shell_id)
        .ok_or_else(|| "暂不支持显示该系统桌面项目菜单".to_string())?;

    shell_context::show_native_context_menu_for_parsing_name(
        window,
        &parsing_name,
        screen_x,
        screen_y,
    )
}

/// 从 Shell 解析名或真实路径识别为受支持的系统桌面图标，供原生拖放层生成稳定引用。
pub fn resolve_shell_id_from_parsing_name(parsing_name: &str) -> Option<&'static str> {
    identity::resolve_shell_id_from_parsing_name(parsing_name)
}

/// Shell 虚拟项路径键与普通文件路径共享排序表，因此构造逻辑必须保持稳定且可逆。
fn create_shell_desktop_item(shell_id: &str) -> Option<DesktopItem> {
    let known_item = catalog::resolve_known_shell_virtual_item(shell_id)?;
    let parsing_name = catalog::resolve_shell_parsing_name(shell_id)?;

    Some(DesktopItem {
        extension: None,
        icon_data_url: shell_icon::resolve_parsing_name_icon_data_url(&parsing_name),
        id: format!("shell_{shell_id}"),
        kind: DesktopItemKind::Shell,
        name: known_item.name.to_string(),
        path: format!("{SHELL_ITEM_PATH_PREFIX}{shell_id}"),
        shell_id: Some(shell_id.to_string()),
        source: DesktopItemSource::Shell,
    })
}
