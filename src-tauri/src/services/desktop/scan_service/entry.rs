//! Box 文件夹扫描条目读取和低成本签名生成。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// 文件项缓存签名只包含会影响展示快照的低成本元数据，避免每轮扫描都访问 Shell 图像工厂。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileItemCacheSignature {
    pub(super) is_dir: bool,
    pub(super) len: u64,
    pub(super) modified_ms: Option<u128>,
}

/// 一次 read_dir 得到的候选项，缓存判断和后续生成展示模型共享这份元数据。
pub(super) struct BoxFolderScanEntry {
    pub(super) path: PathBuf,
    pub(super) path_key: String,
    pub(super) signature: FileItemCacheSignature,
}

/// 统一校验 Box 文件夹存在性，让完整扫描和轻量 revision 使用同一错误语义。
pub(super) fn resolve_existing_box_folder(folder_path: &str) -> io::Result<PathBuf> {
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
pub(super) fn read_box_folder_entries(folder: &Path) -> io::Result<Vec<BoxFolderScanEntry>> {
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

/// 路径缓存键统一使用前端看到的 Windows 字符串，确保排序路径和缓存路径保持一致。
pub(super) fn stable_path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
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
