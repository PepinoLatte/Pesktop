//! 不触发 Shell 弹窗的文件复制和清理工具。

use std::fs;
use std::path::Path;

use crate::infrastructure::filesystem::naming;

/// 复制单个文件或目录；目录复制逐层回滚，避免失败时留下半截目录树。
pub(super) fn copy_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("目标文件已经存在，已停止复制".to_string());
    }

    if source.is_dir() {
        copy_directory_without_shell_prompt(source, target)
    } else if source.is_file() {
        fs::copy(source, target)
            .map(|_| ())
            .map_err(|error| format!("无法复制文件项：{error}"))
    } else {
        Err(format!(
            "无法处理未知类型的文件项：{}",
            naming::display_path(source)
        ))
    }
}

fn copy_directory_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir(target).map_err(|error| format!("无法创建目标文件夹：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("无法读取源文件夹：{error}"))?
    {
        let entry = entry.map_err(|error| format!("无法读取源文件夹内容：{error}"))?;
        let source_child = entry.path();
        let target_child = target.join(entry.file_name());

        if let Err(error) = copy_path_without_shell_prompt(&source_child, &target_child) {
            let _ = remove_path_if_exists(target);
            return Err(error);
        }
    }

    Ok(())
}

/// 删除文件或目录；路径不存在时视为清理成功，便于回滚流程幂等调用。
pub(super) fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("无法清理目标文件夹：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("无法清理目标文件：{error}"))
    }
}
