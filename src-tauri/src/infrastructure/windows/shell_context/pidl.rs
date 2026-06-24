//! Shell PIDL 解析和释放封装，保证右键菜单来源与 Explorer 一致。

use std::path::Path;

use crate::infrastructure::windows::common::wide;
use windows::core::PCWSTR;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::SHParseDisplayName;

/// Shell 分配的 PIDL 资源，Drop 时通过 CoTaskMemFree 释放，避免右键菜单反复打开泄漏。
pub(super) struct ShellPidl {
    pidl: *mut ITEMIDLIST,
}

impl ShellPidl {
    /// 从真实文件路径解析 PIDL，路径仍由 Shell 解释以匹配 Explorer 菜单行为。
    pub(super) fn from_path(path: &Path) -> Result<Self, String> {
        let wide_path = wide::path_null_terminated(path);

        Self::from_wide_parsing_name(&wide_path)
    }

    /// 从 Shell 解析名解析 PIDL，支持 `::{GUID}` 等虚拟桌面项。
    pub(super) fn from_parsing_name(parsing_name: &str) -> Result<Self, String> {
        let wide_parsing_name = wide::null_terminated(parsing_name);

        Self::from_wide_parsing_name(&wide_parsing_name)
    }

    /// 返回原始 PIDL 指针，调用方只在 Shell API 调用期间借用，不接管释放权。
    pub(super) fn as_ptr(&self) -> *mut ITEMIDLIST {
        self.pidl
    }

    fn from_wide_parsing_name(wide_parsing_name: &[u16]) -> Result<Self, String> {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            SHParseDisplayName(PCWSTR(wide_parsing_name.as_ptr()), None, &mut pidl, 0, None)
                .map_err(|error| format!("系统无法解析该文件项：{error}"))?;
        }

        Ok(Self { pidl })
    }
}

impl Drop for ShellPidl {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.pidl as *const _));
        }
    }
}
