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
/// 内容像素的 Alpha 判定阈值，大于该值视为有效内容（0 表示保留任何非零半透明过渡边缘）。
const CONTENT_ALPHA_THRESHOLD: u8 = 0;
/// 裁剪后保留的相对边距比例，避免内容紧贴画布边缘产生压迫感。
const CONTENT_MARGIN_RATIO: f64 = 0.04;
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
    let mut rgba_bitmap = bitmap_to_rgba(bitmap)?;
    if is_fully_transparent(&rgba_bitmap.pixels) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "shell bitmap is fully transparent",
        ));
    }

    remove_solid_background_if_opaque(
        &mut rgba_bitmap.pixels,
        rgba_bitmap.width as usize,
        rgba_bitmap.height as usize,
    );

    let (width, height, pixels) =
        crop_transparent_padding(rgba_bitmap.width as u32, rgba_bitmap.height as u32, &rgba_bitmap.pixels);
    encode_png(width, height, &pixels)
}

/// HICON 的透明度优先从 32 位 Alpha 通道读取；若无半透明过渡则由传统 AND mask 裁切透明边缘。
unsafe fn icon_bitmaps_to_png(color_bitmap: HBITMAP, mask_bitmap: HBITMAP) -> io::Result<Vec<u8>> {
    let mut color = bitmap_to_rgba(color_bitmap)?;
    let has_smooth_alpha = has_smooth_alpha_channel(&color.pixels);
    if !has_smooth_alpha && !mask_bitmap.is_invalid() {
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

    // 如果位图所有像素均完全不透明且四角为白色/近白色底板，执行边缘连通透明化，消除实心白框
    remove_solid_background_if_opaque(
        &mut color.pixels,
        color.width as usize,
        color.height as usize,
    );

    let (width, height, pixels) =
        crop_transparent_padding(color.width as u32, color.height as u32, &color.pixels);
    encode_png(width, height, &pixels)
}

/// 判断位图是否包含真实平滑的 Alpha 通道（存在 0 < alpha < 255 的渐变像素）
fn has_smooth_alpha_channel(rgba: &[u8]) -> bool {
    rgba.chunks_exact(BYTES_PER_PIXEL as usize)
        .any(|pixel| pixel[3] > TRANSPARENT_ALPHA && pixel[3] < OPAQUE_ALPHA)
}

/// 若位图缺乏透明度（全部 alpha == 255）且四周被纯白底衬包围，执行边缘连通漫水消除白框底板
fn remove_solid_background_if_opaque(rgba: &mut [u8], width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    let is_all_opaque = rgba
        .chunks_exact(BYTES_PER_PIXEL as usize)
        .all(|p| p[3] == OPAQUE_ALPHA);
    if !is_all_opaque {
        return;
    }

    let stride = width * BYTES_PER_PIXEL as usize;
    let is_white_pixel = |p: &[u8]| -> bool {
        p[0] >= 246 && p[1] >= 246 && p[2] >= 246
    };

    let top_left = &rgba[0..4];
    let top_right = &rgba[(width - 1) * 4..width * 4];
    let bottom_left = &rgba[(height - 1) * stride..(height - 1) * stride + 4];
    let bottom_right = &rgba[(height - 1) * stride + (width - 1) * 4..(height - 1) * stride + width * 4];

    if !(is_white_pixel(top_left) && is_white_pixel(top_right) && is_white_pixel(bottom_left) && is_white_pixel(bottom_right)) {
        return;
    }

    let mut visited = vec![false; width * height];
    let mut queue = std::collections::VecDeque::new();

    for x in 0..width {
        let idx_top = x;
        if is_white_pixel(&rgba[x * 4..x * 4 + 4]) {
            visited[idx_top] = true;
            queue.push_back((x, 0));
        }
        let idx_bottom = (height - 1) * width + x;
        let offset = (height - 1) * stride + x * 4;
        if is_white_pixel(&rgba[offset..offset + 4]) {
            visited[idx_bottom] = true;
            queue.push_back((x, height - 1));
        }
    }
    for y in 1..height.saturating_sub(1) {
        let idx_left = y * width;
        let offset_l = y * stride;
        if is_white_pixel(&rgba[offset_l..offset_l + 4]) && !visited[idx_left] {
            visited[idx_left] = true;
            queue.push_back((0, y));
        }
        let idx_right = y * width + (width - 1);
        let offset_r = y * stride + (width - 1) * 4;
        if is_white_pixel(&rgba[offset_r..offset_r + 4]) && !visited[idx_right] {
            visited[idx_right] = true;
            queue.push_back((width - 1, y));
        }
    }

    while let Some((cx, cy)) = queue.pop_front() {
        let p_offset = (cy * width + cx) * 4;
        rgba[p_offset + 3] = TRANSPARENT_ALPHA;

        let neighbors = [
            (cx.wrapping_sub(1), cy),
            (cx + 1, cy),
            (cx, cy.wrapping_sub(1)),
            (cx, cy + 1),
        ];

        for (nx, ny) in neighbors {
            if nx < width && ny < height {
                let n_idx = ny * width + nx;
                if !visited[n_idx] {
                    visited[n_idx] = true;
                    let n_offset = (ny * stride) + nx * 4;
                    if is_white_pixel(&rgba[n_offset..n_offset + 4]) {
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }
}

/// 裁掉四周完全透明的背景，并居中归一化为 1:1 正方形画布，保证所有来源的图标视觉大小完全一致。
fn crop_transparent_padding(width: u32, height: u32, pixels: &[u8]) -> (u32, u32, Vec<u8>) {
    let stride = width as usize * BYTES_PER_PIXEL as usize;
    let mut min_x = width as usize;
    let mut min_y = height as usize;
    let mut max_x = 0_usize;
    let mut max_y = 0_usize;
    let mut has_content = false;

    for y in 0..height as usize {
        for x in 0..width as usize {
            let alpha = pixels[y * stride + x * BYTES_PER_PIXEL as usize + 3];
            if alpha > CONTENT_ALPHA_THRESHOLD {
                has_content = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !has_content {
        return (width, height, pixels.to_vec());
    }

    let margin_x = ((max_x - min_x + 1) as f64 * CONTENT_MARGIN_RATIO).ceil() as usize;
    let margin_y = ((max_y - min_y + 1) as f64 * CONTENT_MARGIN_RATIO).ceil() as usize;
    let left = min_x.saturating_sub(margin_x);
    let top = min_y.saturating_sub(margin_y);
    let right = (max_x + 1 + margin_x).min(width as usize);
    let bottom = (max_y + 1 + margin_y).min(height as usize);
    let crop_width = right - left;
    let crop_height = bottom - top;

    let mut cropped = Vec::with_capacity(crop_width * crop_height * BYTES_PER_PIXEL as usize);
    for row in top..bottom {
        let start = row * stride + left * BYTES_PER_PIXEL as usize;
        let end = start + crop_width * BYTES_PER_PIXEL as usize;
        cropped.extend_from_slice(&pixels[start..end]);
    }

    // 归一化为 1:1 正方形画布：以最大边为长，中心对称填充完全透明像素。
    // 使得不同长宽比的图标在前端 object-contain 缩放下拥有完全统一的视觉面积与大小，
    // 彻底解决图标忽大忽小、非正方形被压扁或过小问题。
    let square_size = crop_width.max(crop_height);
    let mut square_pixels = vec![0_u8; square_size * square_size * BYTES_PER_PIXEL as usize];
    let offset_x = (square_size - crop_width) / 2;
    let offset_y = (square_size - crop_height) / 2;

    for row in 0..crop_height {
        let src_start = row * crop_width * BYTES_PER_PIXEL as usize;
        let src_end = src_start + crop_width * BYTES_PER_PIXEL as usize;
        let dst_start = ((row + offset_y) * square_size + offset_x) * BYTES_PER_PIXEL as usize;
        let dst_end = dst_start + crop_width * BYTES_PER_PIXEL as usize;
        square_pixels[dst_start..dst_end].copy_from_slice(&cropped[src_start..src_end]);
    }

    (square_size as u32, square_size as u32, square_pixels)
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
