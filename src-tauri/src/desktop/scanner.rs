use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use super::{DesktopItem, DesktopItemKind, DesktopSnapshot};

/// 读取当前用户桌面路径；文件夹型 Box 只需要删除策略目标目录，不再扫描桌面图标列表。
pub fn scan_desktop() -> io::Result<DesktopSnapshot> {
    let desktop_path = resolve_desktop_path();

    Ok(DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
    })
}

/// 扫描 Box 真实收纳文件夹的直接子项，供 WebView 自绘文件网格使用。
pub fn scan_box_folder(folder_path: &str) -> io::Result<Vec<DesktopItem>> {
    let folder = PathBuf::from(folder_path);
    if !folder.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Box folder does not exist",
        ));
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        items.push(create_desktop_item(&entry.path())?);
    }

    items.sort_by(compare_desktop_items);
    Ok(items)
}

/// 优先使用系统登记的桌面 Known Folder，桌面被 OneDrive 或策略重定向时也能拿到真实目标目录。
#[cfg(target_os = "windows")]
fn resolve_desktop_path() -> PathBuf {
    resolve_windows_known_desktop_path()
        .or_else(resolve_profile_desktop_path)
        .unwrap_or_else(resolve_current_dir)
}

/// 非 Windows 平台没有当前产品语义里的 Explorer 桌面目录，保留用户目录兜底便于开发调试。
#[cfg(not(target_os = "windows"))]
fn resolve_desktop_path() -> PathBuf {
    resolve_profile_desktop_path().unwrap_or_else(resolve_current_dir)
}

/// `USERPROFILE\Desktop` 只作为兜底路径；它不一定等于 Windows 当前真实桌面目录。
fn resolve_profile_desktop_path() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .ok()
        .map(|profile_path| PathBuf::from(profile_path).join("Desktop"))
}

/// 桌面路径无法解析时仍返回当前目录，避免前端初始化被系统目录异常完全阻断。
fn resolve_current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 调用 Windows Shell Known Folder API 读取当前用户真实桌面，覆盖 D 盘、OneDrive 和组策略重定向场景。
#[cfg(target_os = "windows")]
fn resolve_windows_known_desktop_path() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None).ok()? };
    let path_text_result = unsafe { path.to_string() };
    unsafe {
        CoTaskMemFree(Some(path.as_ptr() as *const _));
    }

    path_text_result
        .ok()
        .filter(|path_text| !path_text.is_empty())
        .map(PathBuf::from)
}

/// 将真实文件系统路径转换成前端展示模型；打开、重命名和删除仍以后续路径操作为准。
fn create_desktop_item(path: &Path) -> io::Result<DesktopItem> {
    let metadata = fs::metadata(path)?;
    let is_dir = metadata.is_dir();

    Ok(DesktopItem {
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        icon_data_url: resolve_item_icon_data_url(path),
        id: stable_item_id(path),
        kind: resolve_item_kind(path, is_dir),
        name: resolve_item_name(path),
        path: path.to_string_lossy().to_string(),
    })
}

/// 文件夹优先、名称其次的排序接近 Explorer 默认直觉，避免刷新后项目位置随机跳动。
fn compare_desktop_items(left: &DesktopItem, right: &DesktopItem) -> std::cmp::Ordering {
    let left_is_folder = matches!(left.kind, DesktopItemKind::Folder);
    let right_is_folder = matches!(right.kind, DesktopItemKind::Folder);

    right_is_folder
        .cmp(&left_is_folder)
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
}

/// 根目录这类路径没有 file_name，使用完整路径作为兜底展示名可以避免空白项目。
fn resolve_item_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// 文件类型只用于选择前端后备图标，不参与真实文件的打开方式判断。
fn resolve_item_kind(path: &Path, is_dir: bool) -> DesktopItemKind {
    if is_dir {
        return DesktopItemKind::Folder;
    }

    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => DesktopItemKind::Shortcut,
        Some(_) => DesktopItemKind::File,
        None => DesktopItemKind::Unknown,
    }
}

/// 基于完整路径生成稳定 ID，避免重名文件在前端列表中互相覆盖。
fn stable_item_id(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value
            } else {
                '_'
            }
        })
        .collect()
}

/// 读取系统 Shell 对该路径解析出的默认展示图像，失败时返回空值交给前端占位图标兜底。
#[cfg(target_os = "windows")]
fn resolve_item_icon_data_url(path: &Path) -> Option<String> {
    windows_icon::resolve_shell_image_data_url(path)
        .ok()
        .flatten()
}

/// 非 Windows 平台暂不伪造图标，避免跨平台扫描结果和系统真实图标语义不一致。
#[cfg(not(target_os = "windows"))]
fn resolve_item_icon_data_url(_path: &Path) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
mod windows_icon {
    use std::io;
    use std::mem::size_of;
    use std::path::Path;

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use png::{BitDepth, ColorType, Encoder};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, SIZE};
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SHGetFileInfoW, SHFILEINFOW,
        SHGFI_ICON, SHGFI_LARGEICON, SIIGBF_BIGGERSIZEOK,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    const SHELL_IMAGE_SIZE: i32 = 96;

    /// 按 Windows Shell 默认逻辑提取缩略图或图标，并编码成浏览器可直接渲染的 PNG data URL。
    pub fn resolve_shell_image_data_url(path: &Path) -> io::Result<Option<String>> {
        if let Ok(Some(thumbnail)) = resolve_shell_thumbnail_data_url(path) {
            return Ok(Some(thumbnail));
        }

        resolve_shell_icon_data_url(path)
    }

    /// Explorer 同源的 Shell 图像工厂会优先返回文件缩略图，普通文件则返回系统图标。
    fn resolve_shell_thumbnail_data_url(path: &Path) -> io::Result<Option<String>> {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let wide_path = to_wide_path(path);
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
                    SIIGBF_BIGGERSIZEOK,
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
    fn resolve_shell_icon_data_url(path: &Path) -> io::Result<Option<String>> {
        let wide_path = to_wide_path(path);
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

    /// Windows API 接收 UTF-16 零结尾路径，保留原始路径可让 `.lnk` 走系统快捷方式解析。
    fn to_wide_path(path: &Path) -> Vec<u16> {
        path.to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }

    /// 将 HICON 转换为 PNG 字节；这里显式释放 GDI 对象，避免频繁刷新文件夹时泄漏句柄。
    unsafe fn icon_to_png_rgba(
        icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    ) -> io::Result<Vec<u8>> {
        let mut icon_info = ICONINFO::default();
        GetIconInfo(icon, &mut icon_info)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        let result = bitmap_to_png(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmColor.into());
        let _ = DeleteObject(icon_info.hbmMask.into());

        result
    }

    /// 从彩色位图读取 32 位像素并转为 PNG，保持 Alpha 通道以匹配 Windows 原生图标透明边缘。
    unsafe fn bitmap_to_png(bitmap: HBITMAP) -> io::Result<Vec<u8>> {
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

        encode_png(width as u32, height as u32, &rgba)
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
}
