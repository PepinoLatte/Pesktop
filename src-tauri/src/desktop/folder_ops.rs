use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use windows::core::{Interface, PCWSTR};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Ole::OleInitialize;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellLinkW, SHFileOperationW, ShellExecuteW, ShellLink,
    FOF_ALLOWUNDO, FOS_PICKFOLDERS, FO_DELETE, SHFILEOPSTRUCTW, SIGDN_FILESYSPATH,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const BOX_FOLDER_PREFIX: &str = "box_";
#[cfg(target_os = "windows")]
const HRESULT_ERROR_CANCELLED: i32 = 0x800704C7_u32 as i32;
#[cfg(target_os = "windows")]
pub(crate) type ShellOperationOwner = HWND;
#[cfg(not(target_os = "windows"))]
pub(crate) type ShellOperationOwner = ();

/// 删除文件夹型 Box 时的真实文件处理策略，值与前端 `BoxDeletePolicy` 保持一致
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDeletePolicy {
    MoveContentsToDesktop,
    KeepFolder,
    RecycleFolder,
}

/// 文件拖入 Box 后的真实处理策略，值与前端 `BoxDropAction` 保持一致
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDropAction {
    Copy,
    Move,
    Map,
}

/// 目标目录存在同名文件时的处理策略，默认重命名以避免弹出 Windows 冲突框。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxConflictPolicy {
    Rename,
    Skip,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileTransferKind {
    Copy,
    Move,
    Shortcut,
}

#[derive(Debug, Clone)]
struct FileTransferPlan {
    source: PathBuf,
    destination: PathBuf,
    kind: FileTransferKind,
    replaces_existing: bool,
}

#[derive(Debug, Clone)]
struct CompletedTransfer {
    source: PathBuf,
    destination: PathBuf,
    kind: FileTransferKind,
    backup: Option<PathBuf>,
}

impl BoxDeletePolicy {
    /// 从前端设置字符串解析删除策略，避免危险文件操作接受任意未知值
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "moveContentsToDesktop" => Ok(Self::MoveContentsToDesktop),
            "keepFolder" => Ok(Self::KeepFolder),
            "recycleFolder" => Ok(Self::RecycleFolder),
            _ => Err("未知的 Box 删除策略".to_string()),
        }
    }
}

impl BoxDropAction {
    /// 从前端设置字符串解析拖入策略，避免未知值触发真实文件操作
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "copy" => Ok(Self::Copy),
            "move" => Ok(Self::Move),
            "map" => Ok(Self::Map),
            _ => Err("未知的 Box 拖入处理方式".to_string()),
        }
    }
}

impl BoxConflictPolicy {
    /// 从前端设置字符串解析同名处理策略，避免危险替换逻辑被任意字符串触发。
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "rename" => Ok(Self::Rename),
            "skip" => Ok(Self::Skip),
            "replace" => Ok(Self::Replace),
            _ => Err("未知的同名文件处理方式".to_string()),
        }
    }
}

/// 在收纳根目录下创建单个 Box 对应的真实文件夹，物理目录名由业务 ID 派生而非用户标题派生
pub fn create_box_folder(root_path: &str, folder_name: &str) -> Result<String, String> {
    let root = PathBuf::from(root_path);
    if root_path.trim().is_empty() {
        return Err("请先选择 Box 收纳位置".to_string());
    }
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|error| format!("无法创建收纳根目录：{error}"))?;
    }
    if !root.is_dir() {
        return Err("收纳位置必须是文件夹".to_string());
    }

    let sanitized_folder_name = sanitize_box_folder_name(folder_name)?;
    let folder_path = root.join(sanitized_folder_name);

    fs::create_dir(&folder_path).map_err(|error| {
        format!(
            "无法创建 Box 文件夹 {}：{error}",
            folder_path.to_string_lossy()
        )
    })?;

    Ok(folder_path.to_string_lossy().to_string())
}

/// 删除 Box 前按策略处理真实文件夹；调用方只有在本函数成功后才能删除数据库记录
pub fn delete_box_folder(
    folder_path: &str,
    desktop_path: &str,
    policy: BoxDeletePolicy,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let folder = PathBuf::from(folder_path);
    if !folder.exists() {
        return Ok(());
    }
    if !folder.is_dir() {
        return Err("Box 路径不是文件夹，已停止删除以避免误操作".to_string());
    }

    match policy {
        BoxDeletePolicy::KeepFolder => Ok(()),
        BoxDeletePolicy::MoveContentsToDesktop => {
            move_folder_contents_to_desktop(&folder, desktop_path, conflict_policy)
        }
        BoxDeletePolicy::RecycleFolder => recycle_path(&folder),
    }
}

/// 将单个 Box 的真实文件夹迁移到新的收纳根目录，保留物理目录名和 Box 展示标题的解耦关系。
pub fn migrate_box_folder(folder_path: &str, root_path: &str) -> Result<String, String> {
    let source = PathBuf::from(folder_path);
    if !source.exists() {
        return Err(format!(
            "Box 文件夹不存在，无法迁移：{}",
            source.to_string_lossy()
        ));
    }
    if !source.is_dir() {
        return Err("Box 路径不是文件夹，已停止迁移以避免误操作".to_string());
    }

    let root = PathBuf::from(root_path);
    if root_path.trim().is_empty() {
        return Err("请先选择新的收纳位置".to_string());
    }
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|error| format!("无法创建收纳根目录：{error}"))?;
    }
    if !root.is_dir() {
        return Err("收纳位置必须是文件夹".to_string());
    }

    let folder_name = source
        .file_name()
        .ok_or_else(|| "无法解析 Box 文件夹名称".to_string())?;
    let destination = root.join(folder_name);
    if paths_refer_to_same_entry(&source, &destination)? {
        return Ok(destination.to_string_lossy().to_string());
    }
    if destination.exists() {
        return Err(format!(
            "目标收纳位置已存在同名文件夹，已停止迁移：{}",
            destination.to_string_lossy()
        ));
    }
    if destination_is_inside_source(&source, &root)? {
        return Err("不能把 Box 文件夹迁移到自身内部".to_string());
    }

    move_path_without_shell_prompt(&source, &destination)?;

    Ok(destination.to_string_lossy().to_string())
}

/// 打开真实收纳文件夹，方便用户检查 Box 对应的磁盘位置
pub fn open_folder_in_explorer(folder_path: &str) -> Result<(), String> {
    let folder = Path::new(folder_path);
    if !folder.exists() {
        return Err("Box 文件夹不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(folder)
}

/// 使用系统默认程序打开 Box 文件项，前端不需要理解具体文件类型。
pub fn open_item_with_system_default(path: &str) -> Result<(), String> {
    let item_path = Path::new(path);
    if !item_path.exists() {
        return Err("文件项不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(item_path)
}

/// 将同一目录下的文件项重命名，避免前端拼接路径时跨目录移动真实文件。
pub fn rename_item(path: &str, new_name: &str) -> Result<(), String> {
    let source = PathBuf::from(path);
    if !source.exists() {
        return Err("文件项不存在，无法重命名".to_string());
    }

    let sanitized_name = sanitize_user_file_name(new_name)?;
    let parent = source
        .parent()
        .ok_or_else(|| "无法解析文件所在目录，已停止重命名".to_string())?;
    let destination = parent.join(sanitized_name);
    if destination == source {
        return Ok(());
    }
    if destination.exists() {
        return Err("同名文件已经存在，已停止重命名".to_string());
    }

    fs::rename(&source, &destination).map_err(|error| format!("无法重命名文件项：{error}"))
}

/// 删除 Box 内文件项时进入回收站，保留 Windows 侧恢复能力。
pub fn recycle_items(paths: &[String]) -> Result<(), String> {
    let sources = normalize_existing_drop_paths(paths)?;
    if sources.is_empty() {
        return Ok(());
    }

    recycle_paths(&sources)
}

/// 按当前拖入策略处理外部文件路径；调用方只传真实文件系统路径，不接收 Shell 虚拟对象
pub fn handle_box_dropped_paths(
    folder_path: &str,
    paths: &[String],
    action: BoxDropAction,
    _owner: ShellOperationOwner,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let folder = PathBuf::from(folder_path);
    if !folder.is_dir() {
        return Err("Box 文件夹不存在，无法处理拖入文件".to_string());
    }

    let sources = normalize_existing_drop_paths(paths)?;
    if sources.is_empty() {
        return Err("未读取到可处理的拖入文件".to_string());
    }

    transfer_paths_without_shell_prompts(&sources, &folder, action, conflict_policy)
}

/// 按当前拖出策略处理 Box 内文件到桌面目录；桌面目录缺失时先创建以保持 Shell 语义稳定。
pub fn handle_box_dragged_paths_to_desktop(
    desktop_path: &str,
    paths: &[String],
    action: BoxDropAction,
    _owner: ShellOperationOwner,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    if desktop_path.trim().is_empty() {
        return Err("无法定位桌面路径，已停止拖出文件".to_string());
    }

    let desktop = PathBuf::from(desktop_path);
    if !desktop.exists() {
        fs::create_dir_all(&desktop).map_err(|error| format!("无法创建桌面目录：{error}"))?;
    }
    if !desktop.is_dir() {
        return Err("桌面路径不是文件夹，无法处理拖出文件".to_string());
    }

    let sources = normalize_existing_drop_paths(paths)?;
    if sources.is_empty() {
        return Err("未读取到可拖出的 Box 文件".to_string());
    }

    transfer_paths_without_shell_prompts(&sources, &desktop, action, conflict_policy)
}

/// 文件夹选择器必须使用 Windows Shell 原生对话框，保证用户看到熟悉的目录选择体验
#[cfg(target_os = "windows")]
pub fn choose_collection_root_folder(owner: HWND) -> Result<Option<String>, String> {
    unsafe {
        let _ = OleInitialize(None);
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("无法创建文件夹选择器：{error}"))?;
        dialog
            .SetOptions(FOS_PICKFOLDERS)
            .map_err(|error| format!("无法设置文件夹选择模式：{error}"))?;

        if let Err(error) = dialog.Show(Some(owner)) {
            if error.code().0 == HRESULT_ERROR_CANCELLED {
                return Ok(None);
            }

            return Err(format!("无法打开文件夹选择器：{error}"));
        }

        let item = dialog
            .GetResult()
            .map_err(|error| format!("无法读取选择的文件夹：{error}"))?;
        let path = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| format!("无法解析选择的文件夹路径：{error}"))?;
        let path_text = path
            .to_string()
            .map_err(|error| format!("无法转换选择的文件夹路径：{error}"))?;

        CoTaskMemFree(Some(path.as_ptr() as *const _));
        Ok(Some(path_text))
    }
}

/// 非 Windows 平台没有当前产品目标里的原生文件夹选择语义，保持显式错误避免误判已支持
#[cfg(not(target_os = "windows"))]
pub fn choose_collection_root_folder(_owner: ()) -> Result<Option<String>, String> {
    Err("当前平台暂不支持选择 Box 收纳位置".to_string())
}

fn sanitize_box_folder_name(folder_name: &str) -> Result<String, String> {
    let trimmed = folder_name.trim();
    if !trimmed.starts_with(BOX_FOLDER_PREFIX) {
        return Err("Box 文件夹名必须由 Dasktop 生成".to_string());
    }
    if trimmed
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
    {
        return Ok(trimmed.to_string());
    }

    Err("Box 文件夹名包含非法字符".to_string())
}

fn sanitize_user_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("文件名不能为空".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("文件名不能使用系统保留名称".to_string());
    }
    if trimmed
        .chars()
        .any(|value| matches!(value, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err("文件名包含 Windows 不允许的字符".to_string());
    }

    Ok(trimmed.trim_end_matches('.').trim_end().to_string())
}

fn move_folder_contents_to_desktop(
    folder: &Path,
    desktop_path: &str,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let desktop = PathBuf::from(desktop_path);
    if desktop_path.trim().is_empty() {
        return Err("无法定位桌面路径，已停止删除 Box".to_string());
    }
    if !desktop.exists() {
        fs::create_dir_all(&desktop).map_err(|error| format!("无法创建桌面目录：{error}"))?;
    }

    let children = collect_folder_children(folder)?;
    if !children.is_empty() {
        transfer_paths_without_shell_prompts(
            &children,
            &desktop,
            BoxDropAction::Move,
            conflict_policy,
        )?;
    }

    match fs::remove_dir(folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("收纳文件夹未清空，已保留 Box 记录：{error}")),
    }
}

fn collect_folder_children(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut children = Vec::new();

    for entry in fs::read_dir(folder).map_err(|error| format!("无法读取 Box 文件夹：{error}"))?
    {
        children.push(
            entry
                .map_err(|error| format!("无法读取 Box 文件夹内容：{error}"))?
                .path(),
        );
    }

    Ok(children)
}

fn normalize_existing_drop_paths(paths: &[String]) -> Result<Vec<PathBuf>, String> {
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

/// 普通拖拽传输统一绕开 Shell 文件操作，避免冲突框和权限提升框被 Box 窗口遮挡。
fn transfer_paths_without_shell_prompts(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    let plans = create_transfer_plans(paths, destination, action, conflict_policy)?;
    execute_transfer_plans(&plans)
}

fn create_transfer_plans(
    paths: &[PathBuf],
    destination: &Path,
    action: BoxDropAction,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<FileTransferPlan>, String> {
    let mut reserved_paths = HashSet::new();
    let mut plans = Vec::new();

    for source in paths {
        // 同目录移动没有真实文件变化，提前跳过可以避免后续把自身当成冲突目标处理。
        if action == BoxDropAction::Move && is_direct_child_of_destination(source, destination)? {
            continue;
        }
        // Windows 不允许把文件夹移动到自身内部；提前拦截比等待 fs::rename 报模糊错误更清晰。
        if action == BoxDropAction::Move && destination_is_inside_source(source, destination)? {
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
            desired_shortcut_path(source, destination)?,
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
        && paths_refer_to_same_entry(source, desired_destination)?
    {
        return Ok(None);
    }

    match conflict_policy {
        BoxConflictPolicy::Rename => {
            resolve_renamed_destination(desired_destination, reserved_paths).map(Some)
        }
        BoxConflictPolicy::Skip => {
            if desired_destination.exists() || !reserve_path(desired_destination, reserved_paths)? {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
        BoxConflictPolicy::Replace => {
            // 同一批次内两个来源解析到同一路径时跳过后者，避免“替换”把前一个刚传输的文件覆盖掉。
            if !reserve_path(desired_destination, reserved_paths)? {
                return Ok(None);
            }

            Ok(Some(desired_destination.to_path_buf()))
        }
    }
}

fn resolve_renamed_destination(
    desired_destination: &Path,
    reserved_paths: &mut HashSet<String>,
) -> Result<PathBuf, String> {
    if !desired_destination.exists() && reserve_path(desired_destination, reserved_paths)? {
        return Ok(desired_destination.to_path_buf());
    }

    let parent = desired_destination
        .parent()
        .ok_or_else(|| "无法解析目标文件夹".to_string())?;
    let (stem, extension) = split_duplicate_name(desired_destination);

    for duplicate_index in 2..10_000 {
        let candidate = parent.join(format!("{stem} ({duplicate_index}){extension}"));
        if !candidate.exists() && reserve_path(&candidate, reserved_paths)? {
            return Ok(candidate);
        }
    }

    Err("目标目录存在过多同名文件，无法自动生成新文件名".to_string())
}

fn execute_transfer_plans(plans: &[FileTransferPlan]) -> Result<(), String> {
    let mut completed_transfers = Vec::new();

    for plan in plans {
        // 替换策略先把旧目标改名成隐藏备份，只有整批传输成功后才清理，保证失败可回滚。
        let backup = if plan.replaces_existing {
            prepare_replace_backup(&plan.destination)?
        } else {
            None
        };

        if let Err(error) = execute_single_transfer(plan) {
            let _ = remove_path_if_exists(&plan.destination);
            if let Some(backup_path) = backup {
                let _ = restore_replace_backup(&backup_path, &plan.destination);
            }
            rollback_completed_transfers(&completed_transfers);
            return Err(error);
        }

        completed_transfers.push(CompletedTransfer {
            source: plan.source.clone(),
            destination: plan.destination.clone(),
            kind: plan.kind,
            backup,
        });
    }

    cleanup_replace_backups(&completed_transfers)
}

fn execute_single_transfer(plan: &FileTransferPlan) -> Result<(), String> {
    match plan.kind {
        FileTransferKind::Copy => copy_path_without_shell_prompt(&plan.source, &plan.destination),
        FileTransferKind::Move => move_path_without_shell_prompt(&plan.source, &plan.destination),
        FileTransferKind::Shortcut => create_shortcut_for_path(&plan.source, &plan.destination),
    }
}

fn rollback_completed_transfers(completed_transfers: &[CompletedTransfer]) {
    for transfer in completed_transfers.iter().rev() {
        // 回滚按完成顺序倒序执行，避免目录移动时父子路径互相占用。
        match transfer.kind {
            FileTransferKind::Move => {
                if transfer.destination.exists() {
                    let _ = move_path_without_shell_prompt(&transfer.destination, &transfer.source);
                }
            }
            FileTransferKind::Copy | FileTransferKind::Shortcut => {
                let _ = remove_path_if_exists(&transfer.destination);
            }
        }

        if let Some(backup) = &transfer.backup {
            let _ = restore_replace_backup(backup, &transfer.destination);
        }
    }
}

fn cleanup_replace_backups(completed_transfers: &[CompletedTransfer]) -> Result<(), String> {
    for transfer in completed_transfers {
        if let Some(backup) = &transfer.backup {
            remove_path_if_exists(backup)?;
        }
    }

    Ok(())
}

fn prepare_replace_backup(destination: &Path) -> Result<Option<PathBuf>, String> {
    if !destination.exists() {
        return Ok(None);
    }

    let backup_path = resolve_replace_backup_path(destination)?;
    fs::rename(destination, &backup_path)
        .map_err(|error| format!("无法备份同名目标，已停止替换：{error}"))?;

    Ok(Some(backup_path))
}

fn restore_replace_backup(backup: &Path, destination: &Path) -> Result<(), String> {
    let _ = remove_path_if_exists(destination);
    fs::rename(backup, destination).map_err(|error| format!("无法恢复被替换的同名文件：{error}"))
}

fn resolve_replace_backup_path(destination: &Path) -> Result<PathBuf, String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "无法解析同名目标所在文件夹".to_string())?;
    let file_name = destination
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "item".to_string());
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    for index in 0..10_000 {
        let candidate = parent.join(format!(
            ".dasktop_replace_backup_{}_{}_{}",
            stamp, index, file_name
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("无法创建同名替换备份路径".to_string())
}

fn move_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("目标文件已经存在，已停止移动".to_string());
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if is_cross_device_move_error(&error) => {
            if let Err(copy_error) = copy_path_without_shell_prompt(source, target) {
                let _ = remove_path_if_exists(target);
                return Err(format!("无法跨盘移动文件项：{copy_error}"));
            }
            if let Err(remove_error) = remove_original_after_copy(source) {
                let _ = remove_path_if_exists(target);
                return Err(format!("无法删除原文件，已取消跨盘移动：{remove_error}"));
            }

            Ok(())
        }
        Err(error) => Err(format!(
            "无法移动文件项，可能没有写入权限或文件正在使用：{error}"
        )),
    }
}

fn copy_path_without_shell_prompt(source: &Path, target: &Path) -> Result<(), String> {
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
            source.to_string_lossy()
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

fn remove_original_after_copy(source: &Path) -> Result<(), String> {
    if source.is_dir() {
        fs::remove_dir_all(source).map_err(|error| format!("无法删除原文件夹：{error}"))
    } else {
        fs::remove_file(source).map_err(|error| format!("无法删除原文件：{error}"))
    }
}

fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("无法清理目标文件夹：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("无法清理目标文件：{error}"))
    }
}

/// Windows 跨盘重命名会返回 ERROR_NOT_SAME_DEVICE；这时可以退回复制后删除。
fn is_cross_device_move_error(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(17))
}

fn desired_shortcut_path(source: &Path, destination: &Path) -> Result<PathBuf, String> {
    let stem = source
        .file_stem()
        .or_else(|| source.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "映射项目".to_string());

    Ok(destination.join(format!("{stem}.lnk")))
}

fn split_duplicate_name(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "文件".to_string());
    let extension = path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();

    (stem, extension)
}

fn reserve_path(path: &Path, reserved_paths: &mut HashSet<String>) -> Result<bool, String> {
    Ok(reserved_paths.insert(path_key(path)?))
}

fn path_key(path: &Path) -> Result<String, String> {
    let absolute_path = if path.exists() {
        fs::canonicalize(path).map_err(|error| format!("无法规范化路径：{error}"))?
    } else if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("无法读取当前目录：{error}"))?
            .join(path)
    };
    let text = absolute_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        Ok(text.to_ascii_lowercase())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(text)
    }
}

fn paths_refer_to_same_entry(left: &Path, right: &Path) -> Result<bool, String> {
    if !left.exists() || !right.exists() {
        return Ok(false);
    }

    Ok(path_key(left)? == path_key(right)?)
}

fn is_direct_child_of_destination(source: &Path, destination: &Path) -> Result<bool, String> {
    let Some(parent) = source.parent() else {
        return Ok(false);
    };

    Ok(path_key(parent)? == path_key(destination)?)
}

fn destination_is_inside_source(source: &Path, destination: &Path) -> Result<bool, String> {
    if !source.is_dir() {
        return Ok(false);
    }

    let source_key = path_key(source)?;
    let destination_key = path_key(destination)?;
    let separator = std::path::MAIN_SEPARATOR.to_string();
    let prefix = format!(
        "{}{}",
        source_key.trim_end_matches(std::path::MAIN_SEPARATOR),
        separator
    );

    Ok(destination_key.starts_with(&prefix))
}

#[cfg(target_os = "windows")]
fn recycle_path(path: &Path) -> Result<(), String> {
    recycle_paths(&[path.to_path_buf()])
}

#[cfg(target_os = "windows")]
fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    let mut from = to_double_null_path_list(paths);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_mut_ptr()),
        fFlags: FOF_ALLOWUNDO.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };

    if result != 0 {
        return Err(format!(
            "Windows 无法将 Box 文件项移入回收站，错误码 {result}"
        ));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("已取消删除 Box 文件项".to_string());
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn create_shortcut_for_path(path: &Path, shortcut_path: &Path) -> Result<(), String> {
    if shortcut_path.exists() {
        return Err("目标快捷方式已经存在，已停止映射".to_string());
    }

    let target_wide = to_wide_path(path);
    let shortcut_wide = to_wide_path(shortcut_path);

    unsafe {
        let _ = OleInitialize(None);
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("无法创建映射快捷方式：{error}"))?;
        shell_link
            .SetPath(PCWSTR(target_wide.as_ptr()))
            .map_err(|error| format!("无法写入映射目标：{error}"))?;
        if let Some(parent) = path.parent() {
            let working_directory = to_wide_path(parent);
            shell_link
                .SetWorkingDirectory(PCWSTR(working_directory.as_ptr()))
                .map_err(|error| format!("无法写入映射工作目录：{error}"))?;
        }

        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
        persist_file
            .Save(PCWSTR(shortcut_wide.as_ptr()), true)
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn create_shortcut_for_path(_path: &Path, _shortcut_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持创建 Box 映射快捷方式".to_string())
}

fn is_windows_shortcut(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn recycle_path(path: &Path) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|error| format!("无法删除 Box 文件夹：{error}"))
}

#[cfg(not(target_os = "windows"))]
fn recycle_paths(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        recycle_path(path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn to_double_null_path_list(paths: &[PathBuf]) -> Vec<u16> {
    let mut wide = Vec::new();

    for path in paths {
        wide.extend(path.to_string_lossy().encode_utf16());
        wide.push(0);
    }
    wide.push(0);

    wide
}

#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn open_path_with_system_default(path: &Path) -> Result<(), String> {
    let path_wide = path
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(format!(
            "系统无法打开 Box 文件夹，错误码 {}",
            result.0 as isize
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_path_with_system_default(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开 Box 文件夹".to_string())
}
