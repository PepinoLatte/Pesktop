//! Windows OLE `IDropTarget` 实现，负责接收系统拖放回调并交给事件发送器。

use std::cell::UnsafeCell;
use std::sync::Arc;

use super::data_object::{resolve_native_drop_items, supports_supported_drop_data};
use super::emitter::NativeDropEmitter;
use super::payload::NativeDropItems;
use super::window::client_point_from_screen;
use windows::core::{implement, Ref};
use windows::Win32::Foundation::{HWND, POINTL};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
    DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;

/// 单个 HWND 的 DropTarget 状态；`UnsafeCell` 用于满足 COM 回调的不可变 self 签名。
#[implement(IDropTarget)]
pub(super) struct NativeDropTarget {
    hwnd: HWND,
    emitter: Arc<NativeDropEmitter>,
    cursor_effect: UnsafeCell<DROPEFFECT>,
    enter_is_valid: UnsafeCell<bool>,
}

impl NativeDropTarget {
    /// 绑定 WebView HWND 和共享事件发送器，等待 Windows OLE 调用拖放回调。
    pub(super) fn new(hwnd: HWND, emitter: Arc<NativeDropEmitter>) -> Self {
        Self {
            hwnd,
            emitter,
            cursor_effect: UnsafeCell::new(DROPEFFECT_NONE),
            enter_is_valid: UnsafeCell::new(false),
        }
    }
}

impl NativeDropTarget_Impl {
    /// 接受当前拖放会话并缓存光标效果，后续 `DragOver` 需要持续回写同一个 effect。
    fn accept_drag(&self, cursor_effect: DROPEFFECT, pdw_effect: *mut DROPEFFECT) {
        unsafe {
            *self.enter_is_valid.get() = true;
            *self.cursor_effect.get() = cursor_effect;
            *pdw_effect = cursor_effect;
        }
    }

    /// 拒绝当前拖放会话，同时清理状态，避免无效数据源继续触发 hover/drop 事件。
    fn reject_drag(&self, pdw_effect: *mut DROPEFFECT) {
        self.reset_drag_state();
        unsafe {
            *pdw_effect = DROPEFFECT_NONE;
        }
    }

    /// 重置拖放状态机；`UnsafeCell` 写入集中在这里，降低 COM 回调中的重复 unsafe 代码。
    fn reset_drag_state(&self) {
        unsafe {
            *self.enter_is_valid.get() = false;
            *self.cursor_effect.get() = DROPEFFECT_NONE;
        }
    }

    /// 判断当前拖放会话是否已经通过 `DragEnter` 校验。
    fn enter_is_valid(&self) -> bool {
        unsafe { *self.enter_is_valid.get() }
    }

    /// 读取当前应回写给 Windows 的拖放光标效果。
    fn cursor_effect(&self) -> DROPEFFECT {
        unsafe { *self.cursor_effect.get() }
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for NativeDropTarget_Impl {
    fn DragEnter(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let Some(data_obj) = pDataObj.as_ref() else {
            self.reject_drag(pdwEffect);
            return Ok(());
        };
        let drop_items = resolve_native_drop_items(data_obj);
        if drop_items.is_none() && !supports_supported_drop_data(data_obj) {
            self.reject_drag(pdwEffect);
            return Ok(());
        }

        let position = client_point_from_screen(self.hwnd, pt);
        self.emitter
            .emit_enter(drop_items.unwrap_or_default(), position);
        let cursor_effect = unsafe { *pdwEffect };
        let cursor_effect = resolve_accepted_drop_effect(cursor_effect);
        self.accept_drag(cursor_effect, pdwEffect);

        Ok(())
    }

    fn DragOver(
        &self,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        if self.enter_is_valid() {
            self.emitter
                .emit_over(client_point_from_screen(self.hwnd, pt));
        }

        unsafe {
            *pdwEffect = self.cursor_effect();
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if self.enter_is_valid() {
            self.emitter.emit_leave();
            self.reset_drag_state();
        }

        Ok(())
    }

    fn Drop(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        if self.enter_is_valid() {
            if let Some(data_obj) = pDataObj.as_ref() {
                if let Some(drop_items) = resolve_native_drop_items(data_obj) {
                    self.emitter
                        .emit_drop(drop_items, client_point_from_screen(self.hwnd, pt));
                }
            } else {
                self.emitter.emit_drop(
                    NativeDropItems::default(),
                    client_point_from_screen(self.hwnd, pt),
                );
            }
        }

        self.reset_drag_state();
        unsafe {
            *pdwEffect = resolve_accepted_drop_effect(*pdwEffect);
        }
        Ok(())
    }
}

/// Explorer 可能只允许某一种 effect；按数据源允许的 effect 回写，避免合法文件显示禁用光标。
fn resolve_accepted_drop_effect(allowed_effect: DROPEFFECT) -> DROPEFFECT {
    if allowed_effect.contains(DROPEFFECT_COPY) {
        return DROPEFFECT_COPY;
    }

    if allowed_effect.contains(DROPEFFECT_LINK) {
        return DROPEFFECT_LINK;
    }

    if allowed_effect.contains(DROPEFFECT_MOVE) {
        return DROPEFFECT_MOVE;
    }

    DROPEFFECT_COPY
}
