//! 文件剪贴板操作领域枚举，负责把前端命令字符串解析为受控文件操作。

use std::str::FromStr;

use crate::domain::desktop::contract::file_clipboard_operation_code;

/// 文件剪贴板操作只接受复制和剪切两种意图，对应 Windows Shell 的 Copy/Move DropEffect。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClipboardOperation {
    Copy,
    Cut,
}

impl FromStr for FileClipboardOperation {
    type Err = String;

    /// 从前端快捷键命令解析剪贴板意图，避免未知字符串进入真实文件操作链路。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            file_clipboard_operation_code::COPY => Ok(Self::Copy),
            file_clipboard_operation_code::CUT => Ok(Self::Cut),
            _ => Err("未知的文件剪贴板操作".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文件剪贴板操作会触发复制或移动，解析层必须保持严格白名单。
    #[test]
    fn parses_file_clipboard_operation() {
        assert_eq!(
            "copy".parse::<FileClipboardOperation>(),
            Ok(FileClipboardOperation::Copy)
        );
        assert_eq!(
            "cut".parse::<FileClipboardOperation>(),
            Ok(FileClipboardOperation::Cut)
        );
        assert!("move".parse::<FileClipboardOperation>().is_err());
    }
}
