//! Windows Store 快捷方式 AppUserModelID 解析，覆盖 ShellLink 不暴露普通路径的应用入口。

use std::fs;
use std::io;
use std::path::Path;

/// AppUserModelID 由 `PackageFamilyName!AppId` 组成，分隔符必须显式命名避免扫描逻辑隐式依赖字符。
const APP_USER_MODEL_ID_SEPARATOR: char = '!';
/// 包发布者 ID 过短时容易误判普通文本，保持最低长度可以降低二进制扫描误报。
const MIN_PUBLISHER_ID_LENGTH: usize = 4;
/// UTF-16LE 字符串可能从奇偶任意偏移开始，因此需要尝试两个对齐位置。
const UTF16_ALIGNMENT_OFFSETS: [usize; 2] = [0, 1];

/// 从快捷方式文件中提取 AppUserModelID，读取失败直接上抛给调用方决定是否回退。
pub(super) fn extract_from_shortcut_file(path: &Path) -> io::Result<Option<String>> {
    fs::read(path).map(|bytes| extract_from_bytes(&bytes))
}

/// `.lnk` 可能以 UTF-16LE 或窄字节保存字符串，两个方向都扫描以适配不同 ShellLink 数据块。
fn extract_from_bytes(bytes: &[u8]) -> Option<String> {
    extract_from_utf16le(bytes).or_else(|| extract_from_ascii(bytes))
}

/// UTF-16LE 字符串不保证从偶数偏移开始，分别尝试两个对齐位置以避免漏读属性块内容。
fn extract_from_utf16le(bytes: &[u8]) -> Option<String> {
    UTF16_ALIGNMENT_OFFSETS
        .into_iter()
        .filter(|offset| *offset < bytes.len())
        .find_map(|offset| {
            scan_units(
                bytes[offset..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]) as u32),
            )
        })
}

/// 少数 ShellLink 扩展块会保存窄字节字符串，作为 UTF-16LE 扫描之外的保守兜底。
fn extract_from_ascii(bytes: &[u8]) -> Option<String> {
    scan_units(bytes.iter().map(|byte| *byte as u32))
}

/// 将不同编码统一成字符单元流扫描，避免 UTF-16LE 和窄字节两条路径维护重复状态机。
fn scan_units(units: impl IntoIterator<Item = u32>) -> Option<String> {
    let mut candidate = String::new();
    for unit in units {
        if let Some(app_user_model_id) = scan_unit(unit, &mut candidate) {
            return Some(app_user_model_id);
        }
    }

    take_candidate(&mut candidate)
}

/// 累积可能属于 AUMID 的字符，遇到分隔符时立即校验当前片段并清空状态。
fn scan_unit(unit: u32, candidate: &mut String) -> Option<String> {
    let Some(character) = char::from_u32(unit) else {
        return take_candidate(candidate);
    };
    if is_app_user_model_id_character(character) {
        candidate.push(character);
        return None;
    }

    take_candidate(candidate)
}

/// 只有符合 `PackageFamilyName!AppId` 结构的片段才会被当作 Store 应用入口。
fn take_candidate(candidate: &mut String) -> Option<String> {
    let value = candidate.trim_matches('\0').to_string();
    candidate.clear();

    looks_like_app_user_model_id(&value).then_some(value)
}

/// AppUserModelID 由包族名和应用 ID 组成，过滤普通单词可以避免误把描述文本当作解析名。
fn looks_like_app_user_model_id(value: &str) -> bool {
    let Some((package_family_name, app_id)) = value.split_once(APP_USER_MODEL_ID_SEPARATOR) else {
        return false;
    };
    if package_family_name.is_empty() || app_id.is_empty() || value.contains('\\') {
        return false;
    }

    let Some((package_name, publisher_id)) = package_family_name.rsplit_once('_') else {
        return false;
    };

    !package_name.is_empty()
        && publisher_id.len() >= MIN_PUBLISHER_ID_LENGTH
        && package_name.chars().all(is_app_user_model_id_character)
        && publisher_id.chars().all(is_app_user_model_id_character)
        && app_id.chars().all(is_app_user_model_id_character)
}

/// AUMID 的有效字符集保持窄化，避免二进制扫描跨字段拼出不可解析的 Shell 名称。
fn is_app_user_model_id_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(character, '.' | '_' | '-' | APP_USER_MODEL_ID_SEPARATOR)
}
