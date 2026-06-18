use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{DesktopItem, DesktopItemKind, DesktopSnapshot};

/// 扫描用户桌面目录，返回可用于自绘图标的轻量元信息
pub fn scan_desktop() -> io::Result<DesktopSnapshot> {
    let desktop_path = resolve_desktop_path();
    let mut items = Vec::new();

    if desktop_path.exists() {
        for entry in fs::read_dir(&desktop_path)? {
            let entry = entry?;
            let path = entry.path();

            items.push(create_desktop_item(&path)?);
        }
    }

    items.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    Ok(DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
        items,
    })
}

/// 按真实路径解析项目元信息，支持 Box 收纳任意磁盘文件时继续复用 Windows Shell 图标逻辑
pub fn scan_paths(paths: &[String]) -> io::Result<Vec<DesktopItem>> {
    let mut seen_paths = HashSet::new();
    let mut items = Vec::new();

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        let normalized_key = normalize_path_key(&path);
        if !seen_paths.insert(normalized_key) || !path.exists() {
            continue;
        }

        items.push(create_desktop_item(&path)?);
    }

    Ok(items)
}

/// 将文件系统路径转换成前端可展示的桌面项目；真实打开方式仍由系统 Shell 处理
fn create_desktop_item(path: &Path) -> io::Result<DesktopItem> {
    let metadata = fs::metadata(path)?;

    Ok(DesktopItem {
        id: stable_item_id(path),
        name: resolve_item_name(path),
        path: path.to_string_lossy().to_string(),
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        kind: resolve_item_kind(path, metadata.is_dir()),
        icon_data_url: resolve_item_icon_data_url(path),
    })
}

/// 根目录这类路径没有 file_name，使用完整路径作为展示名可以避免空白项目
fn resolve_item_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// 优先使用 Windows 用户桌面路径，拿不到用户目录时才退回当前目录保证命令可返回
fn resolve_desktop_path() -> PathBuf {
    if let Ok(profile_path) = env::var("USERPROFILE") {
        return PathBuf::from(profile_path).join("Desktop");
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 文件类型只用于选择前端图标，不参与真实文件的打开方式判断
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

/// 基于完整路径生成稳定 ID，避免重名文件在前端列表中互相覆盖
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

/// Windows 文件系统路径大小写不敏感，批量解析时用归一化键去重但不改变返回的真实路径
fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// 读取系统 Shell 对该路径解析出的默认展示图像，失败时返回空值交给前端占位图标兜底
#[cfg(target_os = "windows")]
fn resolve_item_icon_data_url(path: &Path) -> Option<String> {
    windows_icon::resolve_shell_image_data_url(path)
        .ok()
        .flatten()
}

/// 非 Windows 平台暂不伪造图标，避免跨平台扫描结果和系统真实图标语义不一致
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

    /// 按 Windows Shell 默认逻辑提取缩略图或图标，并编码成浏览器可直接渲染的 PNG data URL
    pub fn resolve_shell_image_data_url(path: &Path) -> io::Result<Option<String>> {
        if let Ok(Some(thumbnail)) = resolve_shell_thumbnail_data_url(path) {
            return Ok(Some(thumbnail));
        }

        resolve_shell_icon_data_url(path)
    }

    /// Explorer 同源的 Shell 图像工厂会优先返回文件缩略图，普通文件则返回系统图标
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

    /// Shell 图像工厂不可用时，退回路径对应的大图标，保证每个桌面项都有可识别展示
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

    /// Windows API 接收 UTF-16 零结尾路径，保留原始路径可让 `.lnk` 走系统快捷方式解析
    fn to_wide_path(path: &Path) -> Vec<u16> {
        path.to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }

    /// 将 HICON 转换为 PNG 字节；这里显式释放 GDI 对象，避免频繁刷新桌面时泄漏句柄
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

    /// 从彩色位图读取 32 位像素并转为 PNG，保持 Alpha 通道以匹配 Windows 原生图标透明边缘
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

    /// PNG 编码失败不会中断桌面扫描，上层会回退到语义图标
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

    /// Windows API 失败时携带调用点，便于后续定位具体系统能力问题
    fn last_os_error(operation: &str) -> io::Error {
        let code = unsafe { GetLastError() };
        io::Error::new(
            io::ErrorKind::Other,
            format!("{operation} failed with Windows error {}", code.0),
        )
    }
}
