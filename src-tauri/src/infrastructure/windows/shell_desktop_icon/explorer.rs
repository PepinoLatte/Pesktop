//! Explorer 桌面刷新通知封装。

use std::ffi::c_void;

use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

/// 通知 Explorer 桌面图标配置已变化，触发桌面刷新。
pub(super) fn refresh_explorer_desktop_icons() {
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            None::<*const c_void>,
            None::<*const c_void>,
        );
    }
}
