//! Shell 解析名归一化和业务 ID 匹配逻辑。

use super::catalog::{KNOWN_SHELL_VIRTUAL_ITEMS, USER_FOLDER_SHELL_ID};
use super::user_folder;

/// 从 Shell 解析名或真实路径识别为受支持的系统桌面图标，供原生拖放层生成稳定引用。
pub(super) fn resolve_shell_id_from_parsing_name(parsing_name: &str) -> Option<&'static str> {
    let normalized = normalize_shell_identity(parsing_name);

    if let Some(item) = KNOWN_SHELL_VIRTUAL_ITEMS.iter().find(|item| {
        item.parsing_names
            .iter()
            .any(|parsing_name| shell_identities_match(&normalized, parsing_name))
    }) {
        return Some(item.id);
    }

    user_folder::resolve_user_folder_parsing_name()
        .filter(|profile| shell_identities_match(&normalized, profile))
        .map(|_| USER_FOLDER_SHELL_ID)
}

/// 归一化只用于匹配 Windows 返回的解析名差异，业务层仍保存短小稳定的 shell_id。
fn normalize_shell_identity(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('\\')
        .to_ascii_lowercase()
        .replace('/', "\\")
}

/// Windows 可能返回 `::{DesktopGuid}\::{ItemGuid}` 这类绝对解析名，尾部匹配可覆盖该差异。
fn shell_identities_match(normalized_actual: &str, expected: &str) -> bool {
    let normalized_expected = normalize_shell_identity(expected);

    normalized_actual == normalized_expected
        || normalized_actual.ends_with(&format!("\\{normalized_expected}"))
}
