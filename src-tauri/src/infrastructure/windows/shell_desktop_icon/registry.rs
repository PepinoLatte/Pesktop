//! 系统桌面图标注册表读写封装。

use crate::infrastructure::windows::common::wide;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_VALUE_TYPE,
};
use windows_core::PCWSTR;

/// Explorer 桌面图标配置注册表键，Drop 时自动关闭底层 HKEY。
pub(super) struct RegistryKey {
    hkey: HKEY,
}

impl RegistryKey {
    /// 以只读方式打开现有配置键；键不存在时由调用方根据 Windows 默认状态兜底。
    pub(super) fn open_read(path: &str) -> Result<Self, WIN32_ERROR> {
        let mut hkey = HKEY::default();
        let path = wide::null_terminated(path);
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

    /// 创建或打开写入配置键，确保两个 Explorer 分支都能写入目标值。
    pub(super) fn create_write(path: &str) -> Result<Self, WIN32_ERROR> {
        let mut hkey = HKEY::default();
        let path = wide::null_terminated(path);
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

    /// 读取 DWORD 值；类型或长度不匹配时按缺失处理，让上层回到默认显示状态。
    pub(super) fn query_dword(&self, value_name: &str) -> Result<u32, WIN32_ERROR> {
        let value_name = wide::null_terminated(value_name);
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

    /// 写入 DWORD 显隐值，0 表示显示，1 表示隐藏。
    pub(super) fn set_dword(&self, value_name: &str, value: u32) -> Result<(), WIN32_ERROR> {
        let value_name = wide::null_terminated(value_name);
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

/// 将 Windows 注册表错误统一转换成用户可读信息，保留原始错误码便于定位。
pub(super) fn format_registry_error(action: &str, error: WIN32_ERROR) -> String {
    format!("{action}失败，Windows 错误码：{}", error.0)
}
