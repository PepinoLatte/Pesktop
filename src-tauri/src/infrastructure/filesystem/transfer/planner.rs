//! 文件传输计划器负责把拖拽动作、目标目录和冲突策略解析成可执行计划。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::desktop::{BoxConflictPolicy, BoxDropAction};
use crate::domain::filesystem::WINDOWS_SHORTCUT_EXTENSION;
use crate::infrastructure::filesystem::naming;
use crate::infrastructure::filesystem::transfer::plan::{FileTransferKind, FileTransferPlan};

/// 为一批拖入项目创建执行计划；同目录移动和冲突跳过不会进入最终计划。
pub(super) fn create_transfer_plans(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<FileTransferPlan>, String> {
    let mut reserved_paths = HashSet::new();
    let mut plans = Vec::new();

    for source in paths {
        // 同目录移动没有真实文件变化，提前跳过可以避免后续把自身当成冲突目标处理。
        if action == BoxDropAction::Move
            && naming::is_direct_child_of_destination(source, destination)?
        {
            continue;
        }
        // Windows 不允许把文件夹移动到自身内部；提前拦截比等待 fs::rename 报模糊错误更清晰。
        if action == BoxDropAction::Move
            && naming::destination_is_inside_source(source, destination)?
        {
            return Err("不能把文件夹移动到自身内部".to_string());
        }

        let (kind, desired_destination) =
            resolve_desired_transfer_destination(source, destination, action)?;
        let Some(destination_path) = resolve_conflict_destination(
            source,
            &desired_destination,
            conflict_policy,
            &mut reserved_paths,
        )?
        else {
            continue;
        };

        plans.push(FileTransferPlan {
            source: source.clone(),
            replaces_existing: conflict_policy == BoxConflictPolicy::Replace
                && destination_path.exists(),
            destination: destination_path,
            kind,
        });
    }

    Ok(plans)
}

fn resolve_desired_transfer_destination(
    source: &Path,
    destination: &Path,
    action: BoxDropAction,
) -> Result<(FileTransferKind, PathBuf), String> {
    if action == BoxDropAction::Map && !is_windows_shortcut(source) {
        return Ok((
            FileTransferKind::Shortcut,
            naming::desired_shortcut_path(source, destination),
        ));
    }

    let file_name = source
        .file_name()
        .ok_or_else(|| "无法解析待处理项目名称".to_string())?;
    let kind = match action {
        BoxDropAction::Copy | BoxDropAction::Map => FileTransferKind::Copy,
        BoxDropAction::Move => FileTransferKind::Move,
    };

    Ok((kind, destination.join(file_name)))
}

fn resolve_conflict_destination(
    source: &Path,
    desired_destination: &Path,
    conflict_policy: BoxConflictPolicy,
    reserved_paths: &mut HashSet<String>,
) -> Result<Option<PathBuf>, String> {
    if conflict_policy == BoxConflictPolicy::Replace
        && naming::paths_refer_to_same_entry(source, desired_destination)?
    {
        return Ok(None);
    }

    match conflict_policy {
        BoxConflictPolicy::Rename => {
            naming::resolve_renamed_destination(desired_destination, reserved_paths).map(Some)
        }
        BoxConflictPolicy::Skip => {
            if desired_destination.exists()
                || !naming::reserve_path(desired_destination, reserved_paths)?
            {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
        BoxConflictPolicy::Replace => {
            // 同一批次内两个来源解析到同一路径时跳过后者，避免“替换”把前一个刚传输的文件覆盖掉。
            if !naming::reserve_path(desired_destination, reserved_paths)? {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
    }
}

fn is_windows_shortcut(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case(WINDOWS_SHORTCUT_EXTENSION))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 传输计划测试目录放在系统临时目录下，避免依赖用户真实桌面或 Box 数据。
    struct TempTransferRoot {
        path: PathBuf,
    }

    impl TempTransferRoot {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let path = std::env::temp_dir().join(format!(
                "dasktop_transfer_planner_test_{}_{}",
                std::process::id(),
                stamp
            ));
            fs::create_dir_all(&path).expect("create transfer planner test directory");

            Self { path }
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, name).expect("create transfer planner test file");
            path
        }

        fn folder(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::create_dir_all(&path).expect("create transfer planner test folder");
            path
        }
    }

    impl Drop for TempTransferRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// 映射普通文件会生成快捷方式计划，避免复制源文件破坏“映射”语义。
    #[test]
    fn creates_shortcut_plan_for_map_action() {
        let root = TempTransferRoot::new();
        let source = root.file("source.txt");
        let destination = root.folder("box");

        let plans = create_transfer_plans(
            std::slice::from_ref(&source),
            &destination,
            BoxDropAction::Map,
            BoxConflictPolicy::Rename,
        )
        .expect("create map transfer plan");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].kind, FileTransferKind::Shortcut);
        assert_eq!(plans[0].source, source);
        assert_eq!(plans[0].destination, destination.join("source.lnk"));
    }

    /// 同名跳过策略遇到已存在目标时不会生成执行计划，避免后续覆盖用户文件。
    #[test]
    fn skips_existing_destination_for_skip_policy() {
        let root = TempTransferRoot::new();
        let source = root.file("source.txt");
        let destination = root.folder("box");
        fs::write(destination.join("source.txt"), "existing")
            .expect("create existing destination file");

        let plans = create_transfer_plans(
            &[source],
            &destination,
            BoxDropAction::Copy,
            BoxConflictPolicy::Skip,
        )
        .expect("create skip transfer plan");

        assert!(plans.is_empty());
    }

    /// 重命名策略会为同名目标生成副本名，同时保留原始传输方式。
    #[test]
    fn renames_existing_destination_for_rename_policy() {
        let root = TempTransferRoot::new();
        let source = root.file("source.txt");
        let destination = root.folder("box");
        fs::write(destination.join("source.txt"), "existing")
            .expect("create existing destination file");

        let plans = create_transfer_plans(
            &[source],
            &destination,
            BoxDropAction::Copy,
            BoxConflictPolicy::Rename,
        )
        .expect("create rename transfer plan");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].kind, FileTransferKind::Copy);
        assert!(plans[0].destination.file_name().is_some_and(|file_name| {
            file_name.to_string_lossy().contains("source")
                && file_name.to_string_lossy() != "source.txt"
        }));
    }

    /// 替换策略会标记目标已存在，执行器据此先备份旧目标再提交传输。
    #[test]
    fn marks_existing_destination_for_replace_policy() {
        let root = TempTransferRoot::new();
        let source = root.file("source.txt");
        let destination = root.folder("box");
        fs::write(destination.join("source.txt"), "existing")
            .expect("create existing destination file");

        let plans = create_transfer_plans(
            &[source],
            &destination,
            BoxDropAction::Copy,
            BoxConflictPolicy::Replace,
        )
        .expect("create replace transfer plan");

        assert_eq!(plans.len(), 1);
        assert!(plans[0].replaces_existing);
        assert_eq!(plans[0].destination, destination.join("source.txt"));
    }
}
