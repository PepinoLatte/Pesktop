//! 桌面文件项服务负责 Box 文件夹扫描、文件项操作和 Explorer 原生菜单用例。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::UNIX_EPOCH;

use crate::domain::box_policy::{BoxConflictPolicy, BoxDropAction};
use crate::domain::desktop_item::FileClipboardOperation;
use crate::domain::desktop_item::{
    DesktopItem, DesktopItemKind, DesktopItemSource, DesktopSnapshot,
};
use crate::infrastructure::filesystem::{box_folder as box_folder_fs, transfer};
use crate::infrastructure::windows::{
    desktop_path, shell_clipboard, shell_context, shell_desktop_icon_visibility, shell_file,
    shell_icon, shell_virtual_item,
};

/// Box 文件夹扫描缓存复用未变化文件项的 Shell 图标，避免兜底轮询反复触发昂贵的系统缩略图解析。
static BOX_FOLDER_SCAN_CACHE: OnceLock<Mutex<HashMap<String, FolderScanCache>>> = OnceLock::new();

/// 单个 Box 文件夹的扫描缓存，以完整路径作为文件项稳定键。
#[derive(Default)]
struct FolderScanCache {
    items_by_path: HashMap<String, CachedDesktopItem>,
}

/// 缓存项保存展示模型和文件签名，签名变化时必须重新读取图标和展示字段。
struct CachedDesktopItem {
    item: DesktopItem,
    signature: FileItemCacheSignature,
}

/// 文件项缓存签名只包含会影响展示快照的低成本元数据，避免每轮扫描都访问 Shell 图像工厂。
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileItemCacheSignature {
    is_dir: bool,
    len: u64,
    modified_ms: Option<u128>,
}

/// 一次 read_dir 得到的候选项，缓存判断和后续生成展示模型共享这份元数据。
struct BoxFolderScanEntry {
    path: PathBuf,
    path_key: String,
    signature: FileItemCacheSignature,
}

/// 缓存命中项直接进入排序，未命中项延后到锁外解析 Shell 图标。
enum BoxFolderScanResolution {
    Cached(DesktopItem),
    Pending(BoxFolderScanEntry),
}

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    let desktop_path = desktop_path::resolve_desktop_path();

    Ok(DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
    })
}

/// 扫描 Box 真实收纳文件夹的直接子项，供 WebView 自绘文件网格使用。
pub fn list_box_folder_items(folder_path: &str) -> Result<Vec<DesktopItem>, String> {
    scan_box_folder(folder_path).map_err(|error| error.to_string())
}

/// 计算 Box 文件夹轻量版本号，前端轮询用它判断是否需要拉取完整图标列表。
pub fn get_box_folder_revision(folder_path: &str) -> Result<String, String> {
    calculate_box_folder_revision(folder_path).map_err(|error| error.to_string())
}

/// 根据 Box 持久化的 Shell ID 重建系统桌面图标展示项，未知项会被过滤掉。
pub fn list_shell_desktop_items(shell_ids: &[String]) -> Result<Vec<DesktopItem>, String> {
    Ok(shell_virtual_item::list_shell_virtual_desktop_items(
        shell_ids,
    ))
}

/// 读取 Windows 原生桌面上某个系统图标是否显示，供 Dasktop 接管前记录用户原始状态。
pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String> {
    shell_desktop_icon_visibility::get_shell_desktop_icon_visible(shell_id)
}

/// 设置 Windows 原生桌面上某个系统图标显示状态，Box 内引用仍由前端数据库独立维护。
pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String> {
    shell_desktop_icon_visibility::set_shell_desktop_icon_visible(shell_id, visible)
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致。
pub fn open_desktop_item(path: &str) -> Result<(), String> {
    if let Some(shell_id) = shell_virtual_item::strip_shell_item_path(path) {
        return shell_virtual_item::open_shell_virtual_item(shell_id);
    }

    shell_file::open_item_with_system_default(path)
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
pub fn show_native_item_context_menu(
    window: &tauri::WebviewWindow,
    path: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    if let Some(shell_id) = shell_virtual_item::strip_shell_item_path(path) {
        return shell_virtual_item::show_shell_virtual_item_context_menu(
            window, shell_id, screen_x, screen_y,
        );
    }

    let item_path = Path::new(path);
    if !item_path.exists() {
        return Err("文件项不存在，可能已经被移动或删除".to_string());
    }

    shell_context::show_native_context_menu_for_path(window, item_path, screen_x, screen_y)
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹。
pub fn rename_desktop_item(path: &str, new_name: &str) -> Result<String, String> {
    box_folder_fs::rename_item(path, new_name)
}

/// 删除 Box 内选中文件项时走 Windows 回收站，避免误删后无法恢复。
pub fn delete_desktop_items(paths: &[String]) -> Result<(), String> {
    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Ok(());
    }

    shell_file::recycle_paths(&sources)
}

/// 将 Box 当前选区写入 Windows 文件剪贴板，支持后续跨 Box 或 Explorer 粘贴。
pub fn write_desktop_items_to_clipboard(
    paths: &[String],
    operation: FileClipboardOperation,
) -> Result<(), String> {
    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Ok(());
    }

    shell_clipboard::write_file_list(&sources, operation)
}

/// 从 Windows 文件剪贴板读取路径并粘贴到当前 Box，返回是否执行了文件传输。
pub fn paste_desktop_items_from_clipboard(
    folder_path: &str,
    conflict_policy: BoxConflictPolicy,
) -> Result<bool, String> {
    let Some(payload) = shell_clipboard::read_file_list()? else {
        return Ok(false);
    };
    let action = match payload.operation {
        FileClipboardOperation::Copy => BoxDropAction::Copy,
        FileClipboardOperation::Cut => BoxDropAction::Move,
    };
    let paths = payload
        .paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    box_folder_fs::handle_box_dropped_paths(folder_path, &paths, action, conflict_policy)?;

    Ok(true)
}

fn scan_box_folder(folder_path: &str) -> io::Result<Vec<DesktopItem>> {
    let folder = resolve_existing_box_folder(folder_path)?;

    let folder_cache_key = stable_path_key(&folder);
    let entries = read_box_folder_entries(&folder)?;
    let mut resolutions = Vec::with_capacity(entries.len());
    {
        let mut scan_cache = lock_box_folder_scan_cache();
        let folder_cache = scan_cache.entry(folder_cache_key.clone()).or_default();
        let present_path_keys = entries
            .iter()
            .map(|entry| entry.path_key.clone())
            .collect::<HashSet<_>>();

        folder_cache
            .items_by_path
            .retain(|path_key, _| present_path_keys.contains(path_key));
        for entry in entries {
            if let Some(cached_item) = folder_cache
                .items_by_path
                .get(&entry.path_key)
                .filter(|cached_item| cached_item.signature == entry.signature)
            {
                resolutions.push(BoxFolderScanResolution::Cached(cached_item.item.clone()));
                continue;
            }

            resolutions.push(BoxFolderScanResolution::Pending(entry));
        }
    }

    let mut items = Vec::with_capacity(resolutions.len());
    let mut refreshed_items = Vec::new();
    for resolution in resolutions {
        match resolution {
            BoxFolderScanResolution::Cached(item) => items.push(item),
            BoxFolderScanResolution::Pending(entry) => {
                let item = create_desktop_item(&entry.path, &entry.signature);
                refreshed_items.push((
                    entry.path_key,
                    CachedDesktopItem {
                        item: item.clone(),
                        signature: entry.signature,
                    },
                ));
                items.push(item);
            }
        }
    }
    if !refreshed_items.is_empty() {
        let mut scan_cache = lock_box_folder_scan_cache();
        let folder_cache = scan_cache.entry(folder_cache_key).or_default();
        for (path_key, cached_item) in refreshed_items {
            folder_cache.items_by_path.insert(path_key, cached_item);
        }
    }
    items.sort_by(compare_desktop_items);
    Ok(items)
}

/// 版本号只覆盖文件列表展示会关心的低成本字段，避免普通轮询传输大体积图标数据。
fn calculate_box_folder_revision(folder_path: &str) -> io::Result<String> {
    let folder = resolve_existing_box_folder(folder_path)?;
    let mut entries = read_box_folder_entries(&folder)?;

    entries.sort_by(|left, right| left.path_key.cmp(&right.path_key));
    let mut hash = FNV_OFFSET_BASIS;
    for entry in &entries {
        update_folder_revision_hash(&mut hash, entry);
    }

    Ok(format!("{hash:016x}:{}", entries.len()))
}

/// 统一校验 Box 文件夹存在性，让完整扫描和轻量 revision 使用同一错误语义。
fn resolve_existing_box_folder(folder_path: &str) -> io::Result<PathBuf> {
    let folder = PathBuf::from(folder_path);
    if !folder.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Box folder does not exist",
        ));
    }

    Ok(folder)
}

/// 读取文件夹直接子项及低成本文件元数据，后续缓存判断不再重复访问文件系统。
fn read_box_folder_entries(folder: &Path) -> io::Result<Vec<BoxFolderScanEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::metadata(&path)?;

        entries.push(BoxFolderScanEntry {
            path_key: stable_path_key(&path),
            signature: create_file_item_cache_signature(&metadata),
            path,
        });
    }

    Ok(entries)
}

/// 将真实文件系统路径转换成前端展示模型；打开、重命名和删除仍以后续路径操作为准。
fn create_desktop_item(path: &Path, signature: &FileItemCacheSignature) -> DesktopItem {
    DesktopItem {
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        icon_data_url: shell_icon::resolve_item_icon_data_url(path),
        id: stable_item_id(path),
        kind: resolve_item_kind(path, signature.is_dir),
        name: resolve_item_name(path),
        path: path.to_string_lossy().to_string(),
        shell_id: None,
        source: DesktopItemSource::FileSystem,
    }
}

/// 文件签名用修改时间毫秒值而非 SystemTime 本体，便于跨平台稳定比较和缓存失效。
fn create_file_item_cache_signature(metadata: &fs::Metadata) -> FileItemCacheSignature {
    FileItemCacheSignature {
        is_dir: metadata.is_dir(),
        len: metadata.len(),
        modified_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis()),
    }
}

/// 获取全局扫描缓存，锁中毒时继续取回内部数据，避免一次 panic 让后续扫描全部失败。
fn lock_box_folder_scan_cache() -> MutexGuard<'static, HashMap<String, FolderScanCache>> {
    BOX_FOLDER_SCAN_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 路径缓存键统一使用前端看到的 Windows 字符串，确保排序路径和缓存路径保持一致。
fn stable_path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

/// 将文件项签名写入稳定哈希，确保新增、删除、重命名和内容修改都会改变 revision。
fn update_folder_revision_hash(hash: &mut u64, entry: &BoxFolderScanEntry) {
    update_hash_with_bytes(hash, entry.path_key.as_bytes());
    update_hash_with_bytes(hash, &[u8::from(entry.signature.is_dir)]);
    update_hash_with_bytes(hash, &entry.signature.len.to_le_bytes());
    match entry.signature.modified_ms {
        Some(modified_ms) => {
            update_hash_with_bytes(hash, &[1]);
            update_hash_with_bytes(hash, &modified_ms.to_le_bytes());
        }
        None => update_hash_with_bytes(hash, &[0]),
    }
}

/// FNV-1a 足够用于轮询变更检测，稳定且不引入额外依赖。
fn update_hash_with_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
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
