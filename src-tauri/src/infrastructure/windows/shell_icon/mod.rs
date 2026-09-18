//! Windows Shell 图标和缩略图解析封装，扫描文件夹时失败会交给前端后备图标兜底。

use std::path::Path;

#[cfg(target_os = "windows")]
mod app_user_model_id;
#[cfg(target_os = "windows")]
mod bitmap;
#[cfg(target_os = "windows")]
mod windows_icon;

/// 读取系统 Shell 对该路径解析出的默认展示图像，失败时返回空值交给前端占位图标兜底。
#[cfg(target_os = "windows")]
pub fn resolve_item_icon_data_url(path: &Path) -> Option<String> {
    windows_icon::resolve_shell_image_data_url(path)
        .ok()
        .flatten()
}

/// 读取 Shell 虚拟项解析名对应的系统图像，供无真实路径的桌面系统图标展示使用。
#[cfg(target_os = "windows")]
pub fn resolve_parsing_name_icon_data_url(parsing_name: &str) -> Option<String> {
    windows_icon::resolve_shell_image_data_url_for_parsing_name(parsing_name)
        .ok()
        .flatten()
}

/// 非 Windows 平台暂不伪造图标，避免跨平台扫描结果和系统真实图标语义不一致。
#[cfg(not(target_os = "windows"))]
pub fn resolve_item_icon_data_url(_path: &Path) -> Option<String> {
    None
}

/// 非 Windows 平台没有 Shell 虚拟项图标解析能力，保持空值让前端兜底。
#[cfg(not(target_os = "windows"))]
pub fn resolve_parsing_name_icon_data_url(_parsing_name: &str) -> Option<String> {
    None
}
