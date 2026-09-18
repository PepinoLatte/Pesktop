//! 原生拖放事件发送器，负责把 Windows DropTarget 数据转换成 Tauri 前端事件。

use super::payload::{
    optional_non_empty, resolve_payload_paths, NativeDragPayload, NativeDropItems,
    NativeDropPosition,
};
use crate::domain::app::contract::{
    TAURI_DRAG_DROP_EVENT, TAURI_DRAG_ENTER_EVENT, TAURI_DRAG_LEAVE_EVENT, TAURI_DRAG_OVER_EVENT,
};
use tauri::{AppHandle, Emitter, EventTarget};
use windows::Win32::Foundation::POINT;

/// 事件发送只依赖 app handle 和窗口 label，避免 DropTarget 持有前端对象引用。
pub(super) struct NativeDropEmitter {
    app: AppHandle,
    window_label: String,
}

impl NativeDropEmitter {
    /// 创建针对单个 Box 窗口的事件发送器，后续 DropTarget 复用该对象减少克隆成本。
    pub(super) fn new(app: AppHandle, window_label: String) -> Self {
        Self { app, window_label }
    }

    /// 发送拖入事件；Shell 虚拟项会同时进入 `paths` 和扩展字段，兼容前端既有事件流。
    pub(super) fn emit_enter(&self, payload: NativeDropItems, position: POINT) {
        let paths = resolve_payload_paths(&payload);
        let _ = self.app.emit_to(
            EventTarget::labeled(&self.window_label),
            TAURI_DRAG_ENTER_EVENT,
            NativeDragPayload {
                paths: optional_non_empty(paths),
                position: NativeDropPosition::from_point(position),
                shell_items: optional_non_empty(payload.shell_items),
            },
        );
    }

    /// 拖动移动事件只携带坐标，避免每帧重复解析 IDataObject。
    pub(super) fn emit_over(&self, position: POINT) {
        let _ = self.app.emit_to(
            EventTarget::labeled(&self.window_label),
            TAURI_DRAG_OVER_EVENT,
            NativeDragPayload {
                paths: None,
                position: NativeDropPosition::from_point(position),
                shell_items: None,
            },
        );
    }

    /// 发送释放事件；真正落地的数据以 Drop 阶段重新解析结果为准。
    pub(super) fn emit_drop(&self, payload: NativeDropItems, position: POINT) {
        let paths = resolve_payload_paths(&payload);
        let _ = self.app.emit_to(
            EventTarget::labeled(&self.window_label),
            TAURI_DRAG_DROP_EVENT,
            NativeDragPayload {
                paths: optional_non_empty(paths),
                position: NativeDropPosition::from_point(position),
                shell_items: optional_non_empty(payload.shell_items),
            },
        );
    }

    /// 发送离开事件，让前端清理高亮状态。
    pub(super) fn emit_leave(&self) {
        let _ = self.app.emit_to(
            EventTarget::labeled(&self.window_label),
            TAURI_DRAG_LEAVE_EVENT,
            (),
        );
    }
}
