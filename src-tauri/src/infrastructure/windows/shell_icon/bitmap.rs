//! GDI 图标和位图转 PNG 逻辑集中在这里，避免 Shell 查询流程混入像素处理细节。

use std::io;
use std::mem::size_of;

use crate::infrastructure::windows::common::error;
use png::{BitDepth, ColorType, Encoder};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO};

/// 32 位位图每个像素包含 BGRA 四个字节。
const BYTES_PER_PIXEL: i32 = 4;
/// DIB 读取使用 32 位颜色深度，便于统一转成浏览器可用的 RGBA。
const BITMAP_COLOR_DEPTH_BITS: u16 = 32;
/// Windows DIB 位图平面数固定为 1。
const BITMAP_COLOR_PLANES: u16 = 1;
/// 传统 icon mask 中高亮通道大于该阈值时视为透明。
const ICON_MASK_ALPHA_THRESHOLD: u8 = 127;
/// 完全透明 Alpha 值。
const TRANSPARENT_ALPHA: u8 = 0;
/// 完全不透明 Alpha 值。
const OPAQUE_ALPHA: u8 = 255;

/// GDI 位图统一转成 RGBA 后再交给图标和缩略图分支分别判断透明度语义。
struct RgbaBitmap {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}

/// 将 HICON 转换为 PNG 字节；这里显式释放 GDI 对象，避免频繁刷新文件夹时泄漏句柄。
pub(super) unsafe fn icon_to_png_rgba(icon: HICON) -> io::Result<Vec<u8>> {
    let mut icon_info = ICONINFO::default();
    GetIconInfo(icon, &mut icon_info).map_err(error::io_other)?;

    let result = icon_bitmaps_to_png(icon_info.hbmColor, icon_info.hbmMask);
    if !icon_info.hbmColor.is_invalid() {
        let _ = DeleteObject(icon_info.hbmColor.into());
    }
    if !icon_info.hbmMask.is_invalid() {
        let _ = DeleteObject(icon_info.hbmMask.into());
    }

    result
}

/// 从 Shell 缩略图位图读取 32 位像素并转为 PNG，保持 Alpha 通道以匹配透明边缘。
pub(super) unsafe fn bitmap_to_png(bitmap: HBITMAP) -> io::Result<Vec<u8>> {
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

/// HICON 的透明度可能存放在传统 AND mask 中，AppX/快捷方式图标尤其依赖这条兜底。
unsafe fn icon_bitmaps_to_png(color_bitmap: HBITMAP, mask_bitmap: HBITMAP) -> io::Result<Vec<u8>> {
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
unsafe fn apply_icon_mask_alpha(color: &mut RgbaBitmap, mask_bitmap: HBITMAP) -> io::Result<()> {
    if mask_bitmap.is_invalid() {
        return Ok(());
    }

    let mask = bitmap_to_rgba(mask_bitmap)?;
    if mask.width != color.width || mask.height < color.height {
        return Ok(());
    }

    for y in 0..color.height as usize {
        for x in 0..color.width as usize {
            let color_index = (y * color.width as usize + x) * BYTES_PER_PIXEL as usize;
            let mask_index = (y * mask.width as usize + x) * BYTES_PER_PIXEL as usize;
            let is_transparent = mask.pixels[mask_index] > ICON_MASK_ALPHA_THRESHOLD
                || mask.pixels[mask_index + 1] > ICON_MASK_ALPHA_THRESHOLD
                || mask.pixels[mask_index + 2] > ICON_MASK_ALPHA_THRESHOLD;
            color.pixels[color_index + 3] = if is_transparent {
                TRANSPARENT_ALPHA
            } else {
                OPAQUE_ALPHA
            };
        }
    }

    Ok(())
}

/// 部分 Shell 图标只有 RGB 数据没有可读 mask；此时显示颜色比返回空白图片更符合用户预期。
fn restore_opaque_alpha_for_visible_icon_pixels(rgba: &mut [u8]) {
    if !rgba
        .chunks_exact(BYTES_PER_PIXEL as usize)
        .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
    {
        return;
    }

    for pixel in rgba.chunks_exact_mut(BYTES_PER_PIXEL as usize) {
        pixel[3] = OPAQUE_ALPHA;
    }
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
        return Err(error::last_os_error("GetObjectW"));
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
            biPlanes: BITMAP_COLOR_PLANES,
            biBitCount: BITMAP_COLOR_DEPTH_BITS,
            biCompression: 0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let mut bgra = vec![0_u8; (width * height * BYTES_PER_PIXEL) as usize];
    let device_context = GetDC(None);
    if device_context.is_invalid() {
        return Err(error::last_os_error("GetDC"));
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
        return Err(error::last_os_error("GetDIBits"));
    }

    let mut rgba = bgra;
    for pixel in rgba.chunks_exact_mut(BYTES_PER_PIXEL as usize) {
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
    rgba.chunks_exact(BYTES_PER_PIXEL as usize)
        .all(|pixel| pixel[3] == TRANSPARENT_ALPHA)
}

/// PNG 编码失败不会中断文件夹扫描，上层会回退到语义图标。
fn encode_png(width: u32, height: u32, rgba: &[u8]) -> io::Result<Vec<u8>> {
    let mut png_bytes = Vec::new();
    {
        let mut encoder = Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(error::io_other)?;
        writer.write_image_data(rgba).map_err(error::io_other)?;
    }

    Ok(png_bytes)
}
