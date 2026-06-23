//! Windows Shell 虚拟桌面项封装，负责把“此电脑”等无文件路径对象映射成稳定业务引用。

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::domain::desktop_item::{DesktopItem, DesktopItemKind, DesktopItemSource};
use crate::infrastructure::windows::{shell_context, shell_file, shell_icon};

/// Box 持久化 Shell 虚拟项时使用的路径前缀，前端排序和选择逻辑可继续复用 path 主键。
pub const SHELL_ITEM_PATH_PREFIX: &str = "shell::";

/// Shell 虚拟项进程内展示缓存，避免 Box 文件轮询时重复解析系统图标和用户文件夹路径。
static SHELL_VIRTUAL_ITEM_CACHE: OnceLock<Mutex<HashMap<String, DesktopItem>>> = OnceLock::new();

/// Shell 虚拟项只维护 Windows 系统稳定解析名，展示名走固定中文以匹配当前产品语言。
#[derive(Debug, Clone, Copy)]
struct KnownShellVirtualItem {
    id: &'static str,
    name: &'static str,
    parsing_names: &'static [&'static str],
}

/// 当前支持拖入 Box 的 Windows 桌面系统图标；用户文件夹用 Known Folder 动态解析。
const KNOWN_SHELL_VIRTUAL_ITEMS: &[KnownShellVirtualItem] = &[
    KnownShellVirtualItem {
        id: "this-pc",
        name: "此电脑",
        parsing_names: &["::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"],
    },
    KnownShellVirtualItem {
        id: "recycle-bin",
        name: "回收站",
        parsing_names: &["::{645FF040-5081-101B-9F08-00AA002F954E}"],
    },
    KnownShellVirtualItem {
        id: "network",
        name: "网络",
        parsing_names: &["::{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}"],
    },
    KnownShellVirtualItem {
        id: "control-panel",
        name: "控制面板",
        parsing_names: &[
            "::{26EE0668-A00A-44D7-9371-BEB064C98683}",
            "::{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}",
        ],
    },
];

/// 用户文件夹除真实 Profile 路径外，也可能以桌面 UserFiles CLSID 出现在 Shell 拖放数据中。
const USER_FOLDER_PARSING_NAMES: &[&str] = &["::{59031A47-3F72-44A7-89C5-5595FE6B30EE}"];

/// 根据持久化的 Shell ID 重建前端文件项模型，未知 ID 会被静默丢弃。
pub fn list_shell_virtual_desktop_items(shell_ids: &[String]) -> Vec<DesktopItem> {
    shell_ids
        .iter()
        .filter_map(|shell_id| resolve_cached_shell_desktop_item(shell_id))
        .collect()
}

/// 判断路径键是否为 Dasktop 自己生成的 Shell 虚拟项引用。
pub fn strip_shell_item_path(path: &str) -> Option<&str> {
    path.strip_prefix(SHELL_ITEM_PATH_PREFIX)
}

/// 打开 Shell 虚拟项时使用 Windows Shell 解析名，而不是要求存在真实文件路径。
pub fn open_shell_virtual_item(shell_id: &str) -> Result<(), String> {
    let parsing_name = resolve_shell_parsing_name(shell_id)
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
    let parsing_name = resolve_shell_parsing_name(shell_id)
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
    let normalized = normalize_shell_identity(parsing_name);

    if let Some(item) = KNOWN_SHELL_VIRTUAL_ITEMS.iter().find(|item| {
        item.parsing_names
            .iter()
            .any(|parsing_name| shell_identities_match(&normalized, parsing_name))
    }) {
        return Some(item.id);
    }

    resolve_user_folder_parsing_name()
        .filter(|profile| shell_identities_match(&normalized, profile))
        .map(|_| "user-folder")
        .or_else(|| {
            USER_FOLDER_PARSING_NAMES
                .iter()
                .any(|parsing_name| shell_identities_match(&normalized, parsing_name))
                .then_some("user-folder")
        })
}

/// Shell 虚拟项路径键与普通文件路径共享排序表，因此构造逻辑必须保持稳定且可逆。
fn create_shell_desktop_item(shell_id: &str) -> Option<DesktopItem> {
    let known_item = resolve_known_shell_virtual_item(shell_id)?;
    let parsing_name = resolve_shell_parsing_name(shell_id)?;

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

/// 读取 Shell 虚拟项展示模型；已知项在当前进程生命周期内稳定，可复用图标 data URL 降低轮询成本。
fn resolve_cached_shell_desktop_item(shell_id: &str) -> Option<DesktopItem> {
    {
        let cache = lock_shell_virtual_item_cache();
        if let Some(item) = cache.get(shell_id) {
            return Some(item.clone());
        }
    }

    let item = create_shell_desktop_item(shell_id)?;
    lock_shell_virtual_item_cache().insert(shell_id.to_string(), item.clone());
    Some(item)
}

/// 获取 Shell 虚拟项缓存；锁中毒时继续复用内部数据，保证展示降级不影响文件操作链路。
fn lock_shell_virtual_item_cache() -> MutexGuard<'static, HashMap<String, DesktopItem>> {
    SHELL_VIRTUAL_ITEM_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 已知项静态清单不包含用户文件夹解析名，但仍通过同一个 ID 暴露给前端。
fn resolve_known_shell_virtual_item(shell_id: &str) -> Option<KnownShellVirtualItem> {
    if shell_id == "user-folder" {
        return Some(KnownShellVirtualItem {
            id: "user-folder",
            name: "用户文件夹",
            parsing_names: &[],
        });
    }

    KNOWN_SHELL_VIRTUAL_ITEMS
        .iter()
        .find(|item| item.id == shell_id)
        .copied()
}

/// 用户文件夹在不同账户和重定向配置下路径不同，运行时解析可以避免硬编码本机路径。
fn resolve_shell_parsing_name(shell_id: &str) -> Option<String> {
    if shell_id == "user-folder" {
        return resolve_user_folder_parsing_name();
    }

    KNOWN_SHELL_VIRTUAL_ITEMS
        .iter()
        .find(|item| item.id == shell_id)
        .and_then(|item| item.parsing_names.first().copied())
        .map(str::to_string)
}

/// 归一化只用于匹配 Windows 返回的解析名差异，业务层仍保存短小稳定的 shell_id。
fn normalize_shell_identity(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('\\')
        .to_ascii_lowercase()
        .replace('/', "\\")
}

/// Windows 可能返回 `::{DesktopGuid}\::{ItemGuid}` 这类绝对解析名，尾部匹配可覆盖该差异。
fn shell_identities_match(normalized_actual: &str, expected: &str) -> bool {
    let normalized_expected = normalize_shell_identity(expected);

    normalized_actual == normalized_expected
        || normalized_actual.ends_with(&format!("\\{normalized_expected}"))
}

#[cfg(target_os = "windows")]
fn resolve_user_folder_parsing_name() -> Option<String> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, KF_FLAG_DEFAULT, None).ok()? };
    let value = unsafe { path.to_string().ok() };
    unsafe {
        CoTaskMemFree(Some(path.0 as *const _));
    }

    value
}

#[cfg(not(target_os = "windows"))]
fn resolve_user_folder_parsing_name() -> Option<String> {
    None
}
