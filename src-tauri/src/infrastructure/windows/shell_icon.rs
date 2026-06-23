//! Windows Shell 图标和缩略图解析封装，扫描文件夹时失败会交给前端后备图标兜底。

use std::io;
use std::path::Path;

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

#[cfg(target_os = "windows")]
mod windows_icon {
    use super::*;
    use std::fs;
    use std::mem::size_of;

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use png::{BitDepth, ColorType, Encoder};
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Foundation::{GetLastError, SIZE};
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, IShellLinkW, SHCreateItemFromParsingName, SHGetFileInfoW,
        SHParseDisplayName, ShellLink, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_PIDL,
        SIIGBF, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY, SIIGBF_SCALEUP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    const SHELL_IMAGE_SIZE: i32 = 96;
    const SHORTCUT_TARGET_PATH_BUFFER_LENGTH: usize = 260;
    const APPS_FOLDER_PARSING_NAME_PREFIX: &str = "shell:AppsFolder\\";

    /// GDI 位图统一转成 RGBA 后再交给图标和缩略图分支分别判断透明度语义。
    struct RgbaBitmap {
        width: i32,
        height: i32,
        pixels: Vec<u8>,
    }

    /// 按 Windows Shell 默认逻辑提取缩略图或图标，并编码成浏览器可直接渲染的 PNG data URL。
    pub fn resolve_shell_image_data_url(path: &Path) -> io::Result<Option<String>> {
        if is_shortcut_path(path) {
            if let Ok(Some(icon)) = resolve_shortcut_icon_data_url(path) {
                return Ok(Some(icon));
            }
        }

        resolve_shell_image_data_url_for_parsing_name(&path.to_string_lossy())
    }

    /// Shell 虚拟项与真实路径共用图像工厂，保证系统图标、缩略图和透明边缘一致。
    pub fn resolve_shell_image_data_url_for_parsing_name(
        parsing_name: &str,
    ) -> io::Result<Option<String>> {
        if let Ok(Some(thumbnail)) = resolve_shell_thumbnail_data_url(parsing_name) {
            return Ok(Some(thumbnail));
        }

        resolve_shell_icon_data_url(parsing_name)
    }

    /// 读取 `.lnk` 内部目标；Store/AppX 快捷方式没有普通文件目标，需要从 AUMID 或 PIDL 解析真实图标。
    fn resolve_shortcut_icon_data_url(path: &Path) -> io::Result<Option<String>> {
        // Store 快捷方式命中 AUMID 后不需要加载 ShellLink，先走轻量二进制扫描可以减少普通扫描时的 COM 成本。
        if let Some(app_user_model_id) = extract_shortcut_app_user_model_id(path)? {
            if let Ok(Some(icon)) = resolve_apps_folder_icon_data_url(&app_user_model_id) {
                return Ok(Some(icon));
            }
        }

        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let shortcut_path = to_wide_parsing_name(&path.to_string_lossy());

        let shell_link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        unsafe {
            persist_file
                .Load(PCWSTR(shortcut_path.as_ptr()), STGM_READ)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        }

        // 普通文件快捷方式仍交给路径 Shell 图标逻辑，保留自定义 icon location 和现有表现。
        if shortcut_has_file_target(&shell_link) {
            return Ok(None);
        }

        let pidl = unsafe {
            shell_link
                .GetIDList()
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?
        };
        if pidl.is_null() {
            return Ok(None);
        }

        let result = unsafe { resolve_pidl_icon_data_url(pidl) };
        unsafe {
            CoTaskMemFree(Some(pidl as *const _));
        }

        result
    }

    /// AppsFolder 应用入口使用纯图标模式，避免 Shell 返回带磁贴背景或大内边距的缩略图。
    fn resolve_apps_folder_icon_data_url(app_user_model_id: &str) -> io::Result<Option<String>> {
        let parsing_name = format!("{APPS_FOLDER_PARSING_NAME_PREFIX}{app_user_model_id}");
        if let Ok(Some(icon)) = resolve_shell_icon_only_image_data_url(&parsing_name) {
            return Ok(Some(icon));
        }

        resolve_shell_image_data_url_for_parsing_name(&parsing_name)
    }

    /// 从快捷方式二进制中提取 AppUserModelID，覆盖 WScript/IShellLink 无法暴露目标路径的 Store 应用。
    fn extract_shortcut_app_user_model_id(path: &Path) -> io::Result<Option<String>> {
        fs::read(path).map(|bytes| extract_app_user_model_id_from_bytes(&bytes))
    }

    /// `.lnk` 可能以 UTF-16LE 或窄字节保存字符串，两个方向都扫描以适配不同 ShellLink 数据块。
    fn extract_app_user_model_id_from_bytes(bytes: &[u8]) -> Option<String> {
        extract_app_user_model_id_from_utf16le(bytes)
            .or_else(|| extract_app_user_model_id_from_ascii(bytes))
    }

    /// UTF-16LE 字符串不保证从偶数偏移开始，分别尝试两个对齐位置以避免漏读属性块内容。
    fn extract_app_user_model_id_from_utf16le(bytes: &[u8]) -> Option<String> {
        (0..=1)
            .filter(|offset| *offset < bytes.len())
            .find_map(|offset| {
                scan_app_user_model_id_units(
                    bytes[offset..]
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]) as u32),
                )
            })
    }

    /// 少数 ShellLink 扩展块会保存窄字节字符串，作为 UTF-16LE 扫描之外的保守兜底。
    fn extract_app_user_model_id_from_ascii(bytes: &[u8]) -> Option<String> {
        scan_app_user_model_id_units(bytes.iter().map(|byte| *byte as u32))
    }

    /// 将不同编码统一成字符单元流扫描，避免 UTF-16LE 和窄字节两条路径维护重复状态机。
    fn scan_app_user_model_id_units(units: impl IntoIterator<Item = u32>) -> Option<String> {
        let mut candidate = String::new();
        for unit in units {
            if let Some(app_user_model_id) = scan_app_user_model_id_unit(unit, &mut candidate) {
                return Some(app_user_model_id);
            }
        }

        take_app_user_model_id_candidate(&mut candidate)
    }

    /// 累积可能属于 AUMID 的字符，遇到分隔符时立即校验当前片段并清空状态。
    fn scan_app_user_model_id_unit(unit: u32, candidate: &mut String) -> Option<String> {
        let Some(character) = char::from_u32(unit) else {
            return take_app_user_model_id_candidate(candidate);
        };
        if is_app_user_model_id_character(character) {
            candidate.push(character);
            return None;
        }

        take_app_user_model_id_candidate(candidate)
    }

    /// 只有符合 `PackageFamilyName!AppId` 结构的片段才会被当作 Store 应用入口。
    fn take_app_user_model_id_candidate(candidate: &mut String) -> Option<String> {
        let value = candidate.trim_matches('\0').to_string();
        candidate.clear();

        looks_like_app_user_model_id(&value).then_some(value)
    }

    /// AppUserModelID 由包族名和应用 ID 组成，过滤普通单词可以避免误把描述文本当作解析名。
    fn looks_like_app_user_model_id(value: &str) -> bool {
        let Some((package_family_name, app_id)) = value.split_once('!') else {
            return false;
        };
        if package_family_name.is_empty() || app_id.is_empty() || value.contains('\\') {
            return false;
        }

        let Some((package_name, publisher_id)) = package_family_name.rsplit_once('_') else {
            return false;
        };

        !package_name.is_empty()
            && publisher_id.len() >= 4
            && package_name.chars().all(is_app_user_model_id_character)
            && publisher_id.chars().all(is_app_user_model_id_character)
            && app_id.chars().all(is_app_user_model_id_character)
    }

    /// AUMID 的有效字符集保持窄化，避免二进制扫描跨字段拼出不可解析的 Shell 名称。
    fn is_app_user_model_id_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '!')
    }

    /// `IShellLink::GetPath` 为空时通常代表目标是 Applications 等 Shell 命名空间对象。
    fn shortcut_has_file_target(shell_link: &IShellLinkW) -> bool {
        let mut target_path = vec![0_u16; SHORTCUT_TARGET_PATH_BUFFER_LENGTH];
        if unsafe {
            shell_link
                .GetPath(&mut target_path, std::ptr::null_mut(), 0)
                .is_err()
        } {
            return false;
        }

        target_path.first().copied().unwrap_or_default() != 0
    }

    /// Explorer 同源的 Shell 图像工厂会优先返回文件缩略图，普通文件则返回系统图标。
    fn resolve_shell_thumbnail_data_url(parsing_name: &str) -> io::Result<Option<String>> {
        resolve_shell_item_image_data_url(parsing_name, SIIGBF_BIGGERSIZEOK)
    }

    /// Store 应用图标使用 `ICONONLY | SCALEUP`，绕开缩略图/磁贴语义并让小尺寸图标由 Shell 放大。
    fn resolve_shell_icon_only_image_data_url(parsing_name: &str) -> io::Result<Option<String>> {
        resolve_shell_item_image_data_url(parsing_name, SIIGBF_ICONONLY | SIIGBF_SCALEUP)
    }

    /// 通过 `IShellItemImageFactory` 统一读取缩略图或纯图标，差异只由调用方传入的 Shell flags 决定。
    fn resolve_shell_item_image_data_url(
        parsing_name: &str,
        flags: SIIGBF,
    ) -> io::Result<Option<String>> {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let wide_path = to_wide_parsing_name(parsing_name);
        let image_factory: IShellItemImageFactory = unsafe {
            SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?
        };
        let bitmap = unsafe {
            image_factory
                .GetImage(
                    SIZE {
                        cx: SHELL_IMAGE_SIZE,
                        cy: SHELL_IMAGE_SIZE,
                    },
                    flags,
                )
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?
        };

        let png = unsafe { bitmap_to_png(bitmap) }?;
        unsafe {
            let _ = DeleteObject(bitmap.into());
        }

        Ok(Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(png)
        )))
    }

    /// Shell 图像工厂不可用时退回路径对应的大图标，保证每个文件项都有可识别展示。
    fn resolve_shell_icon_data_url(parsing_name: &str) -> io::Result<Option<String>> {
        if parsing_name.starts_with("::") {
            return resolve_shell_icon_data_url_from_pidl(parsing_name);
        }

        let wide_path = to_wide_parsing_name(parsing_name);
        let mut file_info = SHFILEINFOW::default();
        let flags = SHGFI_ICON | SHGFI_LARGEICON;

        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide_path.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut file_info),
                size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };

        if result == 0 || file_info.hIcon.is_invalid() {
            return Ok(None);
        }

        let pixels = unsafe { icon_to_png_rgba(file_info.hIcon) };
        unsafe {
            let _ = DestroyIcon(file_info.hIcon);
        }

        pixels.map(|png| Some(format!("data:image/png;base64,{}", STANDARD.encode(png))))
    }

    /// `::{GUID}` 这类虚拟项需先转 PIDL 再取图标，否则部分系统不会给出 HICON。
    fn resolve_shell_icon_data_url_from_pidl(parsing_name: &str) -> io::Result<Option<String>> {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let wide_path = to_wide_parsing_name(parsing_name);
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        unsafe {
            SHParseDisplayName(PCWSTR(wide_path.as_ptr()), None, &mut pidl, 0, None)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        }

        let result = unsafe { resolve_pidl_icon_data_url(pidl) };
        unsafe {
            CoTaskMemFree(Some(pidl as *const _));
        }

        result
    }

    /// 通过 PIDL 调用 `SHGetFileInfoW`，覆盖无文件路径的 Shell 命名空间对象。
    unsafe fn resolve_pidl_icon_data_url(pidl: *mut ITEMIDLIST) -> io::Result<Option<String>> {
        let mut file_info = SHFILEINFOW::default();
        let result = SHGetFileInfoW(
            PCWSTR(pidl as *const u16),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut file_info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_PIDL | SHGFI_ICON | SHGFI_LARGEICON,
        );

        if result == 0 || file_info.hIcon.is_invalid() {
            return Ok(None);
        }

        let pixels = icon_to_png_rgba(file_info.hIcon);
        let _ = DestroyIcon(file_info.hIcon);

        pixels.map(|png| Some(format!("data:image/png;base64,{}", STANDARD.encode(png))))
    }

    /// Windows API 接收 UTF-16 零结尾解析名，真实路径和 `::{GUID}` 都走同一转换。
    fn to_wide_parsing_name(parsing_name: &str) -> Vec<u16> {
        parsing_name.encode_utf16().chain(Some(0)).collect()
    }

    /// 将 HICON 转换为 PNG 字节；这里显式释放 GDI 对象，避免频繁刷新文件夹时泄漏句柄。
    unsafe fn icon_to_png_rgba(
        icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    ) -> io::Result<Vec<u8>> {
        let mut icon_info = ICONINFO::default();
        GetIconInfo(icon, &mut icon_info)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        let result = icon_bitmaps_to_png(icon_info.hbmColor, icon_info.hbmMask);
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(icon_info.hbmColor.into());
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask.into());
        }

        result
    }

    /// HICON 的透明度可能存放在传统 AND mask 中，AppX/快捷方式图标尤其依赖这条兜底。
    unsafe fn icon_bitmaps_to_png(
        color_bitmap: HBITMAP,
        mask_bitmap: HBITMAP,
    ) -> io::Result<Vec<u8>> {
        let mut color = bitmap_to_rgba(color_bitmap)?;
        if is_fully_transparent(&color.pixels) {
            apply_icon_mask_alpha(&mut color, mask_bitmap)?;
        }
        if is_fully_transparent(&color.pixels) {
            restore_opaque_alpha_for_visible_icon_pixels(&mut color.pixels);
        }
        if is_fully_transparent(&color.pixels) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "icon bitmap is fully transparent",
            ));
        }

        encode_png(color.width as u32, color.height as u32, &color.pixels)
    }

    /// 传统图标 mask 中白色表示透明、黑色表示不透明，用它补回空 Alpha 通道。
    unsafe fn apply_icon_mask_alpha(
        color: &mut RgbaBitmap,
        mask_bitmap: HBITMAP,
    ) -> io::Result<()> {
        if mask_bitmap.is_invalid() {
            return Ok(());
        }

        let mask = bitmap_to_rgba(mask_bitmap)?;
        if mask.width != color.width || mask.height < color.height {
            return Ok(());
        }

        for y in 0..color.height as usize {
            for x in 0..color.width as usize {
                let color_index = (y * color.width as usize + x) * 4;
                let mask_index = (y * mask.width as usize + x) * 4;
                let is_transparent = mask.pixels[mask_index] > 127
                    || mask.pixels[mask_index + 1] > 127
                    || mask.pixels[mask_index + 2] > 127;
                color.pixels[color_index + 3] = if is_transparent { 0 } else { 255 };
            }
        }

        Ok(())
    }

    /// 部分 Shell 图标只有 RGB 数据没有可读 mask；此时显示颜色比返回空白图片更符合用户预期。
    fn restore_opaque_alpha_for_visible_icon_pixels(rgba: &mut [u8]) {
        if !rgba
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        {
            return;
        }

        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }

    /// 从 Shell 缩略图位图读取 32 位像素并转为 PNG，保持 Alpha 通道以匹配透明边缘。
    unsafe fn bitmap_to_png(bitmap: HBITMAP) -> io::Result<Vec<u8>> {
        let rgba_bitmap = bitmap_to_rgba(bitmap)?;
        if is_fully_transparent(&rgba_bitmap.pixels) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "shell bitmap is fully transparent",
            ));
        }

        encode_png(
            rgba_bitmap.width as u32,
            rgba_bitmap.height as u32,
            &rgba_bitmap.pixels,
        )
    }

    /// 从 GDI 位图读取 32 位 RGBA 像素；调用方按图像来源决定全透明结果是否有效。
    unsafe fn bitmap_to_rgba(bitmap: HBITMAP) -> io::Result<RgbaBitmap> {
        if bitmap.is_invalid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "icon color bitmap is empty",
            ));
        }

        let mut bitmap_meta = BITMAP::default();
        let meta_size = GetObjectW(
            bitmap.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut bitmap_meta as *mut _ as *mut _),
        );
        if meta_size == 0 {
            return Err(last_os_error("GetObjectW"));
        }

        let width = bitmap_meta.bmWidth;
        let height = bitmap_meta.bmHeight;
        if width <= 0 || height <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "icon bitmap size is invalid",
            ));
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..BITMAPINFOHEADER::default()
            },
            ..BITMAPINFO::default()
        };
        let mut bgra = vec![0_u8; (width * height * 4) as usize];
        let device_context = GetDC(None);
        if device_context.is_invalid() {
            return Err(last_os_error("GetDC"));
        }

        let lines = GetDIBits(
            device_context,
            bitmap,
            0,
            height as u32,
            Some(bgra.as_mut_ptr() as *mut _),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );
        let _ = ReleaseDC(None, device_context);
        if lines == 0 {
            return Err(last_os_error("GetDIBits"));
        }

        let mut rgba = bgra;
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        Ok(RgbaBitmap {
            width,
            height,
            pixels: rgba,
        })
    }

    /// 全透明 PNG 会让前端 `<img>` 占位但肉眼看不见，需要在后端主动回退。
    fn is_fully_transparent(rgba: &[u8]) -> bool {
        rgba.chunks_exact(4).all(|pixel| pixel[3] == 0)
    }

    /// 只对 Windows 快捷方式启用 ShellLink 解析，避免普通文件扫描付出额外 COM 成本。
    fn is_shortcut_path(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    }

    /// PNG 编码失败不会中断文件夹扫描，上层会回退到语义图标。
    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> io::Result<Vec<u8>> {
        let mut png_bytes = Vec::new();
        {
            let mut encoder = Encoder::new(&mut png_bytes, width, height);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
            writer
                .write_image_data(rgba)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        }

        Ok(png_bytes)
    }

    /// Windows API 失败时携带调用点，便于后续定位具体系统能力问题。
    fn last_os_error(operation: &str) -> io::Error {
        let code = unsafe { GetLastError() };
        io::Error::new(
            io::ErrorKind::Other,
            format!("{operation} failed with Windows error {}", code.0),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Store 快捷方式的目标常只存在于 `.lnk` 数据块，测试同时覆盖 Unicode 和窄字节两种保存方式。
        #[test]
        fn extracts_app_user_model_id_from_shortcut_bytes() {
            let app_user_model_id = "OpenAI.Codex_2p2nqsd0c76g0!App";
            let mut utf16_bytes = b"prefix".to_vec();
            for unit in app_user_model_id.encode_utf16() {
                utf16_bytes.extend_from_slice(&unit.to_le_bytes());
            }
            utf16_bytes.extend_from_slice(b"\0suffix");

            assert_eq!(
                extract_app_user_model_id_from_bytes(&utf16_bytes).as_deref(),
                Some(app_user_model_id)
            );

            let ascii_bytes = format!("prefix\0{app_user_model_id}\0suffix").into_bytes();
            assert_eq!(
                extract_app_user_model_id_from_bytes(&ascii_bytes).as_deref(),
                Some(app_user_model_id)
            );
        }

        /// 普通描述文本即使包含感叹号，也不能被误判为 AppsFolder 入口。
        #[test]
        fn ignores_non_app_user_model_id_text() {
            assert_eq!(
                extract_app_user_model_id_from_bytes(b"Codex!Shortcut"),
                None
            );
        }
    }
}
