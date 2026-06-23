//! Windows 系统桌面图标显示状态封装，负责按单个 Shell 虚拟项控制 Explorer 原生图标。

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_VALUE_TYPE,
    };
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    use windows_core::PCWSTR;

    const DESKTOP_ICON_VISIBILITY_PATHS: &[&str] = &[
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\HideDesktopIcons\\NewStartPanel",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\HideDesktopIcons\\ClassicStartMenu",
    ];

    /// Shell ID 与 Windows 桌面系统图标 CLSID 的对应关系，必须和 Shell 虚拟项白名单保持一致。
    #[derive(Debug, Clone, Copy)]
    struct ShellDesktopIconEntry {
        clsid: &'static str,
        default_visible: bool,
        shell_id: &'static str,
    }

    const SHELL_DESKTOP_ICON_ENTRIES: &[ShellDesktopIconEntry] = &[
        ShellDesktopIconEntry {
            clsid: "{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
            default_visible: false,
            shell_id: "this-pc",
        },
        ShellDesktopIconEntry {
            clsid: "{645FF040-5081-101B-9F08-00AA002F954E}",
            default_visible: true,
            shell_id: "recycle-bin",
        },
        ShellDesktopIconEntry {
            clsid: "{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}",
            default_visible: false,
            shell_id: "network",
        },
        ShellDesktopIconEntry {
            clsid: "{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}",
            default_visible: false,
            shell_id: "control-panel",
        },
        ShellDesktopIconEntry {
            clsid: "{59031A47-3F72-44A7-89C5-5595FE6B30EE}",
            default_visible: false,
            shell_id: "user-folder",
        },
    ];

    /// 读取某个系统桌面图标当前是否显示；注册表值缺失时回落到 Windows 默认桌面状态。
    pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String> {
        let entry = resolve_shell_desktop_icon_entry(shell_id)?;
        let key = match RegistryKey::open_read(DESKTOP_ICON_VISIBILITY_PATHS[0]) {
            Ok(key) => key,
            Err(error) if error == ERROR_FILE_NOT_FOUND => return Ok(entry.default_visible),
            Err(error) => return Err(format_registry_error("读取系统桌面图标配置", error)),
        };

        match key.query_dword(entry.clsid) {
            Ok(value) => Ok(value == 0),
            Err(error) if error == ERROR_FILE_NOT_FOUND => Ok(entry.default_visible),
            Err(error) => Err(format_registry_error("读取系统桌面图标显示状态", error)),
        }
    }

    /// 设置某个系统桌面图标是否显示，并通知 Explorer 刷新桌面。
    pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String> {
        let entry = resolve_shell_desktop_icon_entry(shell_id)?;
        let hidden_value = if visible { 0 } else { 1 };

        for path in DESKTOP_ICON_VISIBILITY_PATHS {
            let key = RegistryKey::create_write(path)
                .map_err(|error| format_registry_error("打开系统桌面图标配置", error))?;
            key.set_dword(entry.clsid, hidden_value)
                .map_err(|error| format_registry_error("写入系统桌面图标显示状态", error))?;
        }

        refresh_explorer_desktop_icons();
        Ok(())
    }

    fn resolve_shell_desktop_icon_entry(shell_id: &str) -> Result<ShellDesktopIconEntry, String> {
        SHELL_DESKTOP_ICON_ENTRIES
            .iter()
            .find(|entry| entry.shell_id == shell_id)
            .copied()
            .ok_or_else(|| format!("暂不支持管理该系统桌面图标：{shell_id}"))
    }

    /// Explorer 同时存在新旧开始菜单配置分支，写入两处可覆盖 Windows 10/11 与兼容模式差异。
    struct RegistryKey {
        hkey: HKEY,
    }

    impl RegistryKey {
        fn open_read(path: &str) -> Result<Self, windows::Win32::Foundation::WIN32_ERROR> {
            let mut hkey = HKEY::default();
            let path = to_wide_null(path);
            let result = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(path.as_ptr()),
                    None,
                    KEY_READ,
                    &mut hkey,
                )
            };
            if result != ERROR_SUCCESS {
                return Err(result);
            }

            Ok(Self { hkey })
        }

        fn create_write(path: &str) -> Result<Self, windows::Win32::Foundation::WIN32_ERROR> {
            let mut hkey = HKEY::default();
            let path = to_wide_null(path);
            let result = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(path.as_ptr()),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    None,
                    &mut hkey,
                    None,
                )
            };
            if result != ERROR_SUCCESS {
                return Err(result);
            }

            Ok(Self { hkey })
        }

        fn query_dword(
            &self,
            value_name: &str,
        ) -> Result<u32, windows::Win32::Foundation::WIN32_ERROR> {
            let value_name = to_wide_null(value_name);
            let mut value_type = REG_VALUE_TYPE::default();
            let mut data = [0_u8; std::mem::size_of::<u32>()];
            let mut data_size = data.len() as u32;
            let result = unsafe {
                RegQueryValueExW(
                    self.hkey,
                    PCWSTR(value_name.as_ptr()),
                    None,
                    Some(&mut value_type),
                    Some(data.as_mut_ptr()),
                    Some(&mut data_size),
                )
            };
            if result != ERROR_SUCCESS {
                return Err(result);
            }
            if value_type != REG_DWORD || data_size != std::mem::size_of::<u32>() as u32 {
                return Err(ERROR_FILE_NOT_FOUND);
            }

            Ok(u32::from_le_bytes(data))
        }

        fn set_dword(
            &self,
            value_name: &str,
            value: u32,
        ) -> Result<(), windows::Win32::Foundation::WIN32_ERROR> {
            let value_name = to_wide_null(value_name);
            let data = value.to_le_bytes();
            let result = unsafe {
                RegSetValueExW(
                    self.hkey,
                    PCWSTR(value_name.as_ptr()),
                    None,
                    REG_DWORD,
                    Some(&data),
                )
            };
            if result != ERROR_SUCCESS {
                return Err(result);
            }

            Ok(())
        }
    }

    impl Drop for RegistryKey {
        fn drop(&mut self) {
            let _ = unsafe { RegCloseKey(self.hkey) };
        }
    }

    fn refresh_explorer_desktop_icons() {
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED,
                SHCNF_IDLIST,
                None::<*const c_void>,
                None::<*const c_void>,
            );
        }
    }

    fn to_wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn format_registry_error(
        action: &str,
        error: windows::Win32::Foundation::WIN32_ERROR,
    ) -> String {
        format!("{action}失败，Windows 错误码：{}", error.0)
    }
}

#[cfg(not(target_os = "windows"))]
mod windows_impl {
    /// 非 Windows 平台没有 Explorer 系统桌面图标，保持显式错误避免前端误判已同步。
    pub fn get_shell_desktop_icon_visible(_shell_id: &str) -> Result<bool, String> {
        Err("当前平台不支持读取系统桌面图标显示状态".to_string())
    }

    /// 非 Windows 平台没有 Explorer 系统桌面图标，保持显式错误避免前端误判已同步。
    pub fn set_shell_desktop_icon_visible(_shell_id: &str, _visible: bool) -> Result<(), String> {
        Err("当前平台不支持设置系统桌面图标显示状态".to_string())
    }
}

pub use windows_impl::{get_shell_desktop_icon_visible, set_shell_desktop_icon_visible};
