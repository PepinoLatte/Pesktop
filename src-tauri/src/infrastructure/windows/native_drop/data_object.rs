//! Windows IDataObject 拖放数据解析，负责 HDROP、ShellItemArray 和 CIDA 三类来源。

use std::ffi::OsString;
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::ptr;

use super::payload::{push_unique_shell_drop_item, NativeDropItems, NativeShellDropItem};
use crate::infrastructure::windows::shell_virtual_item;
use windows::Win32::System::Com::{
    CoTaskMemFree, IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{ReleaseStgMedium, CF_HDROP};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    DragQueryFileW, ILCombine, ILFree, IShellItemArray, SHCreateShellItemArrayFromDataObject,
    SHGetNameFromIDList, CFSTR_SHELLIDLIST, HDROP, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_FILESYSPATH,
};

/// `FORMATETC.lindex` 对非多页数据固定为 -1，命名后避免散落裸值。
const FORMAT_ALL_ITEMS_INDEX: i32 = -1;
/// `DragQueryFileW` 用 u32::MAX 查询 HDROP 文件数量。
const HDROP_QUERY_FILE_COUNT_INDEX: u32 = u32::MAX;
/// CIDA 头至少包含 item count 和父 PIDL 偏移两个 u32。
const CIDA_MINIMUM_OFFSET_COUNT: usize = 2;
/// CIDA 第一个偏移指向父 PIDL，后续偏移指向子项 PIDL。
const CIDA_PARENT_OFFSET_INDEX: usize = 0;
/// `CFSTR_SHELLIDLIST` 注册剪贴板格式需要塞入 FORMATETC 的 16 位字段。
const MAX_FORMATETC_CLIPBOARD_FORMAT: u32 = u16::MAX as u32;

/// DropTarget 只接收真实路径或已知 Shell 虚拟项，避免不支持的数据源显示可投放光标。
pub(super) fn supports_supported_drop_data(data_obj: &IDataObject) -> bool {
    supports_hdrop_data(data_obj)
        || supports_shell_idlist_data(data_obj)
        || resolve_shell_drop_items(data_obj).is_some()
}

/// 从 IDataObject 中解析 Dasktop 支持的真实路径和 Shell 虚拟项。
pub(super) fn resolve_native_drop_items(data_obj: &IDataObject) -> Option<NativeDropItems> {
    let mut shell_items = resolve_shell_drop_items(data_obj).unwrap_or_default();
    for shell_item in resolve_shell_idlist_drop_items(data_obj).unwrap_or_default() {
        push_unique_shell_drop_item(&mut shell_items, &shell_item.shell_id);
    }
    let paths = resolve_hdrop_paths(data_obj)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| {
            if let Some(shell_id) = shell_virtual_item::resolve_shell_id_from_parsing_name(path) {
                push_unique_shell_drop_item(&mut shell_items, shell_id);
                return false;
            }

            true
        })
        .collect();
    let drop_items = NativeDropItems { paths, shell_items };

    (!drop_items.is_empty()).then_some(drop_items)
}

fn hglobal_format(clipboard_format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: clipboard_format,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: FORMAT_ALL_ITEMS_INDEX,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn supports_hdrop_data(data_obj: &IDataObject) -> bool {
    let format = hglobal_format(CF_HDROP.0);

    unsafe { data_obj.QueryGetData(&format).is_ok() }
}

/// DragEnter 阶段部分 Explorer 数据源只承诺格式可用，真正内容到 Drop 阶段才稳定返回。
fn supports_shell_idlist_data(data_obj: &IDataObject) -> bool {
    let Some(clipboard_format) = shell_idlist_clipboard_format() else {
        return false;
    };
    let format = hglobal_format(clipboard_format);

    unsafe { data_obj.QueryGetData(&format).is_ok() }
}

fn resolve_hdrop_paths(data_obj: &IDataObject) -> Option<Vec<String>> {
    let format = hglobal_format(CF_HDROP.0);
    let mut medium = unsafe { data_obj.GetData(&format).ok()? };
    let hdrop = HDROP(unsafe { medium.u.hGlobal.0 } as _);
    let item_count = unsafe { DragQueryFileW(hdrop, HDROP_QUERY_FILE_COUNT_INDEX, None) };
    let mut paths = Vec::new();

    for index in 0..item_count {
        let character_count = unsafe { DragQueryFileW(hdrop, index, None) } as usize;
        if character_count == 0 {
            continue;
        }

        let mut buffer = vec![0; character_count + 1];
        unsafe {
            DragQueryFileW(hdrop, index, Some(&mut buffer));
        }
        paths.push(
            OsString::from_wide(&buffer[..character_count])
                .to_string_lossy()
                .to_string(),
        );
    }

    unsafe {
        ReleaseStgMedium(&mut medium);
    }

    (!paths.is_empty()).then_some(paths)
}

/// Windows 自带桌面图标通常没有 CF_HDROP，需从 Shell Item Array 中读取解析名再匹配白名单。
fn resolve_shell_drop_items(data_obj: &IDataObject) -> Option<Vec<NativeShellDropItem>> {
    let item_array: IShellItemArray =
        unsafe { SHCreateShellItemArrayFromDataObject(data_obj).ok()? };
    let item_count = unsafe { item_array.GetCount().ok()? };
    let mut shell_items = Vec::new();

    for index in 0..item_count {
        let Ok(shell_item) = (unsafe { item_array.GetItemAt(index) }) else {
            continue;
        };
        let parsing_names = [
            unsafe { shell_item_display_name(&shell_item, SIGDN_DESKTOPABSOLUTEPARSING) },
            unsafe { shell_item_display_name(&shell_item, SIGDN_FILESYSPATH) },
        ];
        let Some(shell_id) = parsing_names.iter().flatten().find_map(|parsing_name| {
            shell_virtual_item::resolve_shell_id_from_parsing_name(parsing_name)
        }) else {
            continue;
        };

        push_unique_shell_drop_item(&mut shell_items, shell_id);
    }

    (!shell_items.is_empty()).then_some(shell_items)
}

/// Explorer 桌面系统图标常使用 `Shell IDList Array`，需要手动解析 CIDA 中的父 PIDL 和子 PIDL。
fn resolve_shell_idlist_drop_items(data_obj: &IDataObject) -> Option<Vec<NativeShellDropItem>> {
    let clipboard_format = shell_idlist_clipboard_format()?;
    let format = hglobal_format(clipboard_format);
    let mut medium = unsafe { data_obj.GetData(&format).ok()? };
    let hglobal = unsafe { medium.u.hGlobal };
    let locked = unsafe { GlobalLock(hglobal) };
    if locked.is_null() {
        unsafe {
            ReleaseStgMedium(&mut medium);
        }
        return None;
    }

    let size = unsafe { GlobalSize(hglobal) };
    let shell_items = unsafe { resolve_shell_idlist_items_from_memory(locked.cast(), size) };
    let _ = unsafe { GlobalUnlock(hglobal) };
    unsafe {
        ReleaseStgMedium(&mut medium);
    }

    shell_items
}

/// `CFSTR_SHELLIDLIST` 是注册剪贴板格式，集中转换成 `FORMATETC` 需要的 16 位编号。
fn shell_idlist_clipboard_format() -> Option<u16> {
    let clipboard_format = unsafe { RegisterClipboardFormatW(CFSTR_SHELLIDLIST) };
    if clipboard_format == 0 || clipboard_format > MAX_FORMATETC_CLIPBOARD_FORMAT {
        return None;
    }

    Some(clipboard_format as u16)
}

/// CIDA 内存布局为 cidl + (cidl + 1) 个 u32 偏移，第 0 个 PIDL 是父目录，后续是子项。
unsafe fn resolve_shell_idlist_items_from_memory(
    base: *const u8,
    size: usize,
) -> Option<Vec<NativeShellDropItem>> {
    let minimum_header_size = size_of::<u32>() * CIDA_MINIMUM_OFFSET_COUNT;
    if size < minimum_header_size {
        return None;
    }

    let item_count = ptr::read_unaligned(base.cast::<u32>()) as usize;
    let offset_count = item_count.checked_add(1)?;
    let header_size = size_of::<u32>().checked_add(offset_count.checked_mul(size_of::<u32>())?)?;
    if item_count == 0 || header_size > size {
        return None;
    }

    let mut offsets = Vec::with_capacity(offset_count);
    for index in 0..offset_count {
        let offset_pointer = base.add(size_of::<u32>() + index * size_of::<u32>());
        let offset = ptr::read_unaligned(offset_pointer.cast::<u32>()) as usize;
        if offset >= size {
            return None;
        }
        offsets.push(offset);
    }

    let parent_pidl = base
        .add(offsets[CIDA_PARENT_OFFSET_INDEX])
        .cast::<ITEMIDLIST>();
    let mut shell_items = Vec::new();
    for child_offset in offsets.iter().skip(1) {
        let child_pidl = base.add(*child_offset).cast::<ITEMIDLIST>();
        let absolute_pidl = ILCombine(Some(parent_pidl), Some(child_pidl));
        if absolute_pidl.is_null() {
            continue;
        }

        if let Some(parsing_name) = pidl_display_name(absolute_pidl) {
            if let Some(shell_id) =
                shell_virtual_item::resolve_shell_id_from_parsing_name(&parsing_name)
            {
                push_unique_shell_drop_item(&mut shell_items, shell_id);
            }
        }
        ILFree(Some(absolute_pidl));
    }

    (!shell_items.is_empty()).then_some(shell_items)
}

/// PIDL 名称由 Shell 分配，需要释放 PWSTR，避免拖放系统图标时泄漏内存。
unsafe fn pidl_display_name(pidl: *const ITEMIDLIST) -> Option<String> {
    let display_name = SHGetNameFromIDList(pidl, SIGDN_DESKTOPABSOLUTEPARSING).ok()?;
    let value = display_name.to_string().ok();
    CoTaskMemFree(Some(display_name.0 as *const _));

    value
}

/// Shell 分配的 PWSTR 由调用方释放，避免持续拖放系统图标时泄漏 COM 内存。
unsafe fn shell_item_display_name(
    shell_item: &windows::Win32::UI::Shell::IShellItem,
    sigdn: windows::Win32::UI::Shell::SIGDN,
) -> Option<String> {
    let display_name = shell_item.GetDisplayName(sigdn).ok()?;
    let value = display_name.to_string().ok();
    CoTaskMemFree(Some(display_name.0 as *const _));

    value
}
