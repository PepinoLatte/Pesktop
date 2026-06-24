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
            unsafe {
                *self.enter_is_valid.get() = false;
                *self.cursor_effect.get() = DROPEFFECT_NONE;
                *pdwEffect = DROPEFFECT_NONE;
            }
            return Ok(());
        };
        let drop_items = resolve_native_drop_items(data_obj);
        if drop_items.is_none() && !supports_supported_drop_data(data_obj) {
            unsafe {
                *self.enter_is_valid.get() = false;
                *self.cursor_effect.get() = DROPEFFECT_NONE;
                *pdwEffect = DROPEFFECT_NONE;
            }
            return Ok(());
        }

        let position = client_point_from_screen(self.hwnd, pt);
        self.emitter
            .emit_enter(drop_items.unwrap_or_default(), position);
        let cursor_effect = unsafe { resolve_accepted_drop_effect(*pdwEffect) };
        unsafe {
            *self.enter_is_valid.get() = true;
            *self.cursor_effect.get() = cursor_effect;
            *pdwEffect = cursor_effect;
        }

        Ok(())
    }

    fn DragOver(
        &self,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        if unsafe { *self.enter_is_valid.get() } {
            self.emitter
                .emit_over(client_point_from_screen(self.hwnd, pt));
        }

        unsafe {
            *pdwEffect = *self.cursor_effect.get();
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if unsafe { *self.enter_is_valid.get() } {
            self.emitter.emit_leave();
            unsafe {
                *self.enter_is_valid.get() = false;
                *self.cursor_effect.get() = DROPEFFECT_NONE;
            }
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
        if unsafe { *self.enter_is_valid.get() } {
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

        unsafe {
            *self.enter_is_valid.get() = false;
            *self.cursor_effect.get() = DROPEFFECT_NONE;
            *pdwEffect = resolve_accepted_drop_effect(*pdwEffect);
        }
        Ok(())
    }
}

/// Explorer 可能只允许某一种 effect；按数据源允许的 effect 回写，避免合法文件显示禁用光标。
unsafe fn resolve_accepted_drop_effect(allowed_effect: DROPEFFECT) -> DROPEFFECT {
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
