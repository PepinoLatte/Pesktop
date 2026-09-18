//! Box 文件夹轻量 revision 计算。

use std::io;

use super::entry::{self, BoxFolderScanEntry};

/// FNV-1a offset basis，revision 只用于变化检测，不承担安全哈希职责。
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
/// FNV-1a prime，和 offset basis 配套保证跨平台稳定输出。
const FNV_PRIME: u64 = 0x00000100000001b3;

/// 版本号只覆盖文件列表展示会关心的低成本字段，避免普通轮询传输大体积图标数据。
pub(super) fn calculate_box_folder_revision(folder_path: &str) -> io::Result<String> {
    let folder = entry::resolve_existing_box_folder(folder_path)?;
    let mut entries = entry::read_box_folder_entries(&folder)?;

    entries.sort_by(|left, right| left.path_key.cmp(&right.path_key));
    let mut hash = FNV_OFFSET_BASIS;
    for entry in &entries {
        update_folder_revision_hash(&mut hash, entry);
    }

    Ok(format!("{hash:016x}:{}", entries.len()))
}

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
