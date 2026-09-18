//! 支持的 Windows Shell 虚拟桌面项目录，避免展示、拖放和注册表控制各自维护魔法字符串。

use super::user_folder;

/// 用户文件夹稳定业务 ID；Windows 解析名需要运行时按当前用户解析。
pub const USER_FOLDER_SHELL_ID: &str = "user-folder";

/// Shell 虚拟项只维护 Windows 系统稳定解析名，展示名走固定中文以匹配当前产品语言。
#[derive(Debug, Clone, Copy)]
pub(crate) struct KnownShellVirtualItem {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) parsing_names: &'static [&'static str],
    pub(crate) desktop_icon_clsid: Option<&'static str>,
    pub(crate) default_visible: bool,
}

/// 当前支持拖入 Box 的 Windows 桌面系统图标；用户文件夹用 Known Folder 动态解析。
pub(crate) const KNOWN_SHELL_VIRTUAL_ITEMS: &[KnownShellVirtualItem] = &[
    KnownShellVirtualItem {
        id: "this-pc",
        name: "此电脑",
        parsing_names: &["::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"],
        desktop_icon_clsid: Some("{20D04FE0-3AEA-1069-A2D8-08002B30309D}"),
        default_visible: false,
    },
    KnownShellVirtualItem {
        id: "recycle-bin",
        name: "回收站",
        parsing_names: &["::{645FF040-5081-101B-9F08-00AA002F954E}"],
        desktop_icon_clsid: Some("{645FF040-5081-101B-9F08-00AA002F954E}"),
        default_visible: true,
    },
    KnownShellVirtualItem {
        id: "network",
        name: "网络",
        parsing_names: &["::{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}"],
        desktop_icon_clsid: Some("{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}"),
        default_visible: false,
    },
    KnownShellVirtualItem {
        id: "control-panel",
        name: "控制面板",
        parsing_names: &[
            "::{26EE0668-A00A-44D7-9371-BEB064C98683}",
            "::{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}",
        ],
        desktop_icon_clsid: Some("{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}"),
        default_visible: false,
    },
    KnownShellVirtualItem {
        id: USER_FOLDER_SHELL_ID,
        name: "用户文件夹",
        parsing_names: &["::{59031A47-3F72-44A7-89C5-5595FE6B30EE}"],
        desktop_icon_clsid: Some("{59031A47-3F72-44A7-89C5-5595FE6B30EE}"),
        default_visible: false,
    },
];

/// 根据业务 Shell ID 获取静态目录项，未知项返回 None 供上层自然过滤。
pub(crate) fn resolve_known_shell_virtual_item(shell_id: &str) -> Option<KnownShellVirtualItem> {
    KNOWN_SHELL_VIRTUAL_ITEMS
        .iter()
        .find(|item| item.id == shell_id)
        .copied()
}

/// 按 Shell ID 解析 Explorer 可打开或弹菜单的解析名，用户文件夹优先使用真实 Profile 路径。
pub(crate) fn resolve_shell_parsing_name(shell_id: &str) -> Option<String> {
    if shell_id == USER_FOLDER_SHELL_ID {
        return user_folder::resolve_user_folder_parsing_name();
    }

    resolve_known_shell_virtual_item(shell_id)
        .and_then(|item| item.parsing_names.first().copied())
        .map(str::to_string)
}

/// 解析可受注册表显示控制的系统桌面图标，过滤没有 CLSID 的内部项。
pub(crate) fn resolve_desktop_icon_item(shell_id: &str) -> Option<KnownShellVirtualItem> {
    resolve_known_shell_virtual_item(shell_id).filter(|item| item.desktop_icon_clsid.is_some())
}
