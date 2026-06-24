//! 文件剪贴板操作领域枚举，负责把前端命令字符串解析为受控文件操作。

use std::str::FromStr;

use crate::domain::desktop::contract::file_clipboard_operation_code;

/// 文件剪贴板操作只接受复制和剪切两种意图，对应 Windows Shell 的 Copy/Move DropEffect。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClipboardOperation {
    Copy,
    Cut,
}

impl FileClipboardOperation {
    /// 返回前端快捷键命令使用的剪贴板操作代码，避免复制/剪切协议字符串散落。
    pub const fn code(self) -> &'static str {
        match self {
            Self::Copy => file_clipboard_operation_code::COPY,
            Self::Cut => file_clipboard_operation_code::CUT,
        }
    }

    /// 剪切在 Windows 文件剪贴板里对应移动语义，粘贴到 Box 时应转为真实移动操作。
    pub const fn is_cut(self) -> bool {
        matches!(self, Self::Cut)
    }
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
