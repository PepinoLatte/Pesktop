//! 文件传输基础设施统一处理复制、移动、映射、同名冲突和失败回滚。

mod backup;
mod copy;
mod executor;
mod plan;
mod planner;
mod rollback;

use std::path::{Path, PathBuf};

use crate::domain::desktop::{BoxConflictPolicy, BoxDropAction};

/// 过滤并规范化外部传入的真实路径，任何不存在的路径都会阻止本批次继续执行。
pub(crate) fn normalize_existing_paths(paths: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut normalized_paths = Vec::new();

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        if !path.exists() {
            return Err(format!("拖入项目不存在：{}", path.to_string_lossy()));
        }
        normalized_paths.push(path);
    }

    Ok(normalized_paths)
}

/// 普通拖拽传输会预先解析冲突目标，移动交给 Windows Shell 执行以同步刷新 Explorer 桌面视图。
/// 返回实际完成的目标路径，供前端按释放位置写入手动排序；冲突跳过的项目不会出现在结果中。
pub(crate) fn transfer_paths_without_shell_prompts(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<PathBuf>, String> {
    let plans = planner::create_transfer_plans(paths, destination, action, conflict_policy)?;
    executor::execute_transfer_plans(&plans)
}

/// 移动单个路径；内部使用 Shell 文件操作，让 Explorer 立即收到桌面文件增删事件。
pub(crate) fn move_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    executor::move_path_without_shell_prompt(source, target)
}
