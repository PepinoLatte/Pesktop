//! 原生拖放事件载荷模型，保持与 Tauri 内置拖放事件兼容。

use serde::Serialize;
use windows::Win32::Foundation::POINT;

use crate::infrastructure::windows::shell_virtual_item;

/// Drag/drop 事件载荷保持和 Tauri 内置 `onDragDropEvent` 同形，前端无需额外分支。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeDragPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) paths: Option<Vec<String>>,
    pub(super) position: NativeDropPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) shell_items: Option<Vec<NativeShellDropItem>>,
}

/// Shell 虚拟拖放项只暴露 Dasktop 支持的稳定 ID，避免前端保存系统 PIDL 细节。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeShellDropItem {
    pub(super) shell_id: String,
}

/// Tauri 前端会把 position 包成 PhysicalPosition，因此这里保留 x/y 数值结构。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeDropPosition {
    x: f64,
    y: f64,
}

/// 原生拖放解析结果同时承载真实文件路径和受支持的 Shell 虚拟桌面项。
#[derive(Default)]
pub(super) struct NativeDropItems {
    pub(super) paths: Vec<String>,
    pub(super) shell_items: Vec<NativeShellDropItem>,
}

impl NativeDropItems {
    /// 判断拖放数据是否包含 Dasktop 支持的真实路径或 Shell 虚拟项。
    pub(super) fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.shell_items.is_empty()
    }
}

impl NativeDropPosition {
    /// 将 Windows client point 转成前端拖放事件使用的浮点坐标。
    pub(super) fn from_point(point: POINT) -> Self {
        Self {
            x: point.x as f64,
            y: point.y as f64,
        }
    }
}

/// 空 Vec 在事件载荷中省略，保持与 Tauri 原生 payload 的可选字段语义一致。
pub(super) fn optional_non_empty<T>(values: Vec<T>) -> Option<Vec<T>> {
    (!values.is_empty()).then_some(values)
}

/// Tauri 前端 `onDragDropEvent` 会规整 payload 并丢弃未知字段，因此 Shell 项也必须写入标准 paths。
pub(super) fn resolve_payload_paths(payload: &NativeDropItems) -> Vec<String> {
    let mut paths = payload.paths.clone();
    for shell_item in &payload.shell_items {
        paths.push(format!(
            "{}{}",
            shell_virtual_item::SHELL_ITEM_PATH_PREFIX,
            shell_item.shell_id
        ));
    }

    paths
}

/// 一个拖放数据对象可能同时提供 Shell Item 和 HDROP，按 shell_id 去重避免前端重复保存。
pub(super) fn push_unique_shell_drop_item(
    shell_items: &mut Vec<NativeShellDropItem>,
    shell_id: &str,
) {
    if shell_items
        .iter()
        .any(|item: &NativeShellDropItem| item.shell_id == shell_id)
    {
        return;
    }

    shell_items.push(NativeShellDropItem {
        shell_id: shell_id.to_string(),
    });
}
