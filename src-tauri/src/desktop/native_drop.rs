#[cfg(target_os = "windows")]
mod windows_drop {
    use serde::Serialize;
    use std::cell::RefCell;
    use std::cell::UnsafeCell;
    use std::collections::HashMap;
    use std::ffi::{c_void, OsString};
    use std::mem::size_of;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use std::sync::Arc;
    use tauri::{AppHandle, Emitter, EventTarget, Manager};
    use windows::core::{implement, Ref};
    use windows::Win32::Foundation::{HGLOBAL, HWND, LPARAM, POINT, POINTL};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Ole::{
        IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium,
        RevokeDragDrop, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
        DROPEFFECT_NONE,
    };
    use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        DragQueryFileW, ILCombine, ILFree, SHGetNameFromIDList, CFSTR_SHELLIDLIST, CIDA, HDROP,
        SIGDN_DESKTOPABSOLUTEPARSING,
    };
    use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;

    const DRAG_ENTER_EVENT: &str = "tauri://drag-enter";
    const DRAG_OVER_EVENT: &str = "tauri://drag-over";
    const DRAG_DROP_EVENT: &str = "tauri://drag-drop";
    const DRAG_LEAVE_EVENT: &str = "tauri://drag-leave";
    thread_local! {
        /// OLE DropTarget 必须在窗口线程注册和释放，因此控制器保存在 UI 线程本地注册表中
        static NATIVE_DROP_CONTROLLERS: RefCell<HashMap<String, NativeDropController>> =
            RefCell::new(HashMap::new());
    }

    /// Drag/drop 事件载荷保持和 Tauri 内置 `onDragDropEvent` 完全同形，前端无需额外分支
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeDragPayload {
        #[serde(skip_serializing_if = "Option::is_none")]
        paths: Option<Vec<String>>,
        position: NativeDropPosition,
    }

    /// Tauri 前端会把 position 包成 PhysicalPosition，因此这里保留 x/y 数值结构
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeDropPosition {
        x: f64,
        y: f64,
    }

    /// DropTarget 注册结果保存每个 WebView 子 HWND 的 COM 对象，避免被 Rust 提前释放
    struct NativeDropController {
        targets: Vec<RegisteredDropTarget>,
    }

    struct RegisteredDropTarget {
        hwnd: isize,
        target: IDropTarget,
    }

    impl Drop for NativeDropController {
        fn drop(&mut self) {
            for target in &self.targets {
                let _ = unsafe { RevokeDragDrop(HWND(target.hwnd as *mut c_void)) };
                let _ = &target.target;
            }
        }
    }

    /// 为指定 Box 窗口注册支持 Shell IDList 的原生拖放目标；重复注册会替换旧目标
    pub fn register_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
        let app_handle = app.clone();
        let window_label = window_label.to_string();
        app.run_on_main_thread(move || {
            if let Err(error) = register_box_native_drop_on_main_thread(&app_handle, &window_label)
            {
                eprintln!("failed to register native drop target for {window_label}: {error}");
            }
        })
        .map_err(|error| format!("无法调度原生拖放注册：{error}"))
    }

    /// 前端卸载和窗口销毁都会调用注销；未注册时保持幂等，避免关闭路径误报
    pub fn unregister_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
        let window_label = window_label.to_string();
        app.run_on_main_thread(move || {
            NATIVE_DROP_CONTROLLERS.with(|controllers| {
                controllers.borrow_mut().remove(&window_label);
            });
        })
        .map_err(|error| format!("无法调度原生拖放注销：{error}"))
    }

    fn register_box_native_drop_on_main_thread(
        app: &AppHandle,
        window_label: &str,
    ) -> Result<(), String> {
        let webview_window = app
            .get_webview_window(window_label)
            .ok_or_else(|| "Box 窗口尚未创建，无法注册原生拖放".to_string())?;
        let hwnd = webview_window
            .hwnd()
            .map_err(|error| format!("无法获取 Box 窗口句柄：{error}"))?;
        let controller = NativeDropController::new(hwnd, app.clone(), window_label.to_string())?;
        if controller.targets.is_empty() {
            return Err("未找到可注册拖放的 WebView 子窗口".to_string());
        }

        NATIVE_DROP_CONTROLLERS.with(|controllers| {
            controllers
                .borrow_mut()
                .insert(window_label.to_string(), controller);
        });

        Ok(())
    }

    impl NativeDropController {
        fn new(hwnd: HWND, app: AppHandle, window_label: String) -> Result<Self, String> {
            let _ = unsafe { OleInitialize(None) };
            let mut child_windows = vec![hwnd];
            enumerate_child_windows(hwnd, &mut child_windows);

            let mut targets = Vec::new();
            let emitter = Arc::new(NativeDropEmitter { app, window_label });
            for child_hwnd in child_windows {
                let target: IDropTarget =
                    NativeDropTarget::new(child_hwnd, Arc::clone(&emitter)).into();
                let _ = unsafe { RevokeDragDrop(child_hwnd) };

                if unsafe { RegisterDragDrop(child_hwnd, &target) }.is_ok() {
                    targets.push(RegisteredDropTarget {
                        hwnd: child_hwnd.0 as isize,
                        target,
                    });
                }
            }

            Ok(Self { targets })
        }
    }

    /// 事件发送只依赖 app handle 和窗口 label，避免 DropTarget 持有前端对象引用
    struct NativeDropEmitter {
        app: AppHandle,
        window_label: String,
    }

    impl NativeDropEmitter {
        fn emit_enter(&self, paths: Vec<String>, position: POINT) {
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_ENTER_EVENT,
                NativeDragPayload {
                    paths: Some(paths),
                    position: NativeDropPosition::from_point(position),
                },
            );
        }

        fn emit_over(&self, position: POINT) {
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_OVER_EVENT,
                NativeDragPayload {
                    paths: None,
                    position: NativeDropPosition::from_point(position),
                },
            );
        }

        fn emit_drop(&self, paths: Vec<String>, position: POINT) {
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_DROP_EVENT,
                NativeDragPayload {
                    paths: Some(paths),
                    position: NativeDropPosition::from_point(position),
                },
            );
        }

        fn emit_leave(&self) {
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_LEAVE_EVENT,
                (),
            );
        }
    }

    impl NativeDropPosition {
        fn from_point(point: POINT) -> Self {
            Self {
                x: point.x as f64,
                y: point.y as f64,
            }
        }
    }

    #[implement(IDropTarget)]
    struct NativeDropTarget {
        hwnd: HWND,
        emitter: Arc<NativeDropEmitter>,
        cursor_effect: UnsafeCell<DROPEFFECT>,
        enter_is_valid: UnsafeCell<bool>,
    }

    impl NativeDropTarget {
        fn new(hwnd: HWND, emitter: Arc<NativeDropEmitter>) -> Self {
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
            let paths = resolve_drop_paths(data_obj);
            if paths.is_none() && !supports_drop_data(data_obj) {
                unsafe {
                    *self.enter_is_valid.get() = false;
                    *self.cursor_effect.get() = DROPEFFECT_NONE;
                    *pdwEffect = DROPEFFECT_NONE;
                }
                return Ok(());
            }

            let position = client_point_from_screen(self.hwnd, pt);
            self.emitter.emit_enter(paths.unwrap_or_default(), position);
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
                    if let Some(paths) = resolve_drop_paths(data_obj) {
                        self.emitter
                            .emit_drop(paths, client_point_from_screen(self.hwnd, pt));
                    }
                } else {
                    self.emitter
                        .emit_drop(Vec::new(), client_point_from_screen(self.hwnd, pt));
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

    /// Explorer 可能只允许虚拟对象创建链接；按数据源允许的 effect 回写，避免合法 Shell 项仍显示禁用光标
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

    /// 优先解析普通文件路径，失败后再尝试 Shell IDList，保证文件拖放行为不退化
    fn resolve_drop_paths(data_obj: &IDataObject) -> Option<Vec<String>> {
        resolve_hdrop_paths(data_obj).or_else(|| resolve_shell_idlist_paths(data_obj))
    }

    /// Explorer 对部分虚拟项会先声明格式可用，真正数据到 Drop 阶段才稳定返回；DragEnter 只做能力判断
    fn supports_drop_data(data_obj: &IDataObject) -> bool {
        supports_hdrop_data(data_obj) || supports_shell_idlist_data(data_obj)
    }

    fn supports_hdrop_data(data_obj: &IDataObject) -> bool {
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };

        unsafe { data_obj.QueryGetData(&format).is_ok() }
    }

    fn supports_shell_idlist_data(data_obj: &IDataObject) -> bool {
        let Some(cf_shell_idlist) = shell_idlist_clipboard_format() else {
            return false;
        };
        let format = FORMATETC {
            cfFormat: cf_shell_idlist,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };

        unsafe { data_obj.QueryGetData(&format).is_ok() }
    }

    fn resolve_hdrop_paths(data_obj: &IDataObject) -> Option<Vec<String>> {
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium = unsafe { data_obj.GetData(&format).ok()? };
        let hdrop = HDROP(unsafe { medium.u.hGlobal.0 } as _);
        let item_count = unsafe { DragQueryFileW(hdrop, 0xFFFFFFFF, None) };
        let mut paths = Vec::new();

        for index in 0..item_count {
            let character_count = unsafe { DragQueryFileW(hdrop, index, None) } as usize;
            if character_count == 0 {
                continue;
            }

            let mut buffer = vec![0; character_count + 1];
            unsafe {
                DragQueryFileW(hdrop, index, Some(&mut buffer));
            }
            paths.push(
                OsString::from_wide(&buffer[..character_count])
                    .to_string_lossy()
                    .to_string(),
            );
        }

        unsafe {
            ReleaseStgMedium(&mut medium);
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }

    fn resolve_shell_idlist_paths(data_obj: &IDataObject) -> Option<Vec<String>> {
        let cf_shell_idlist = shell_idlist_clipboard_format()?;

        let format = FORMATETC {
            cfFormat: cf_shell_idlist,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium = unsafe { data_obj.GetData(&format).ok()? };
        let hglobal = unsafe { medium.u.hGlobal };
        let paths = unsafe { parse_shell_idlist_hglobal(hglobal) };
        unsafe {
            ReleaseStgMedium(&mut medium);
        }

        paths
    }

    fn shell_idlist_clipboard_format() -> Option<u16> {
        let cf_shell_idlist = unsafe { RegisterClipboardFormatW(CFSTR_SHELLIDLIST) };
        if cf_shell_idlist == 0 || cf_shell_idlist > u16::MAX as u32 {
            return None;
        }

        Some(cf_shell_idlist as u16)
    }

    /// CFSTR_SHELLIDLIST 使用 CIDA 结构存储父 PIDL 和子 PIDL 偏移，需组合后再取 Shell parsing name
    unsafe fn parse_shell_idlist_hglobal(hglobal: HGLOBAL) -> Option<Vec<String>> {
        if hglobal.is_invalid() {
            return None;
        }

        let data_size = GlobalSize(hglobal);
        let data_ptr = GlobalLock(hglobal);
        if data_ptr.is_null() {
            return None;
        }

        let result = parse_shell_idlist_data(data_ptr as *const u8, data_size);
        let _ = GlobalUnlock(hglobal);

        result
    }

    unsafe fn parse_shell_idlist_data(data: *const u8, data_size: usize) -> Option<Vec<String>> {
        if data_size < size_of::<CIDA>() {
            return None;
        }

        let cida = data as *const CIDA;
        let item_count = ptr::read_unaligned(ptr::addr_of!((*cida).cidl)) as usize;
        let offsets_len = item_count.checked_add(1)?;
        let offsets_size = offsets_len.checked_mul(size_of::<u32>())?;
        let header_size = size_of::<u32>().checked_add(offsets_size)?;
        if data_size < header_size {
            return None;
        }

        let offsets_ptr = ptr::addr_of!((*cida).aoffset) as *const u32;
        let parent_offset = read_cida_offset(offsets_ptr, 0);
        if parent_offset >= data_size {
            return None;
        }

        let parent_pidl = data.add(parent_offset) as *const ITEMIDLIST;
        let mut paths = Vec::new();
        for index in 0..item_count {
            let child_offset = read_cida_offset(offsets_ptr, index + 1);
            if child_offset >= data_size {
                continue;
            }

            let child_pidl = data.add(child_offset) as *const ITEMIDLIST;
            let absolute_pidl = ILCombine(Some(parent_pidl), Some(child_pidl));
            if absolute_pidl.is_null() {
                continue;
            }

            if let Some(path) = resolve_supported_shell_item_path(absolute_pidl) {
                paths.push(path);
            }
            ILFree(Some(absolute_pidl as *const ITEMIDLIST));
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }

    /// CIDA 是 packed 结构，偏移数组按非对齐方式读取，避免在 Rust 中触发未定义行为
    unsafe fn read_cida_offset(offsets_ptr: *const u32, index: usize) -> usize {
        ptr::read_unaligned(offsets_ptr.add(index)) as usize
    }

    /// 仅把当前产品支持的 Windows 原生桌面图标映射为稳定 `shell::` 键，其它虚拟对象先不伪装支持
    unsafe fn resolve_supported_shell_item_path(pidl: *const ITEMIDLIST) -> Option<String> {
        let parsing_name = SHGetNameFromIDList(pidl, SIGDN_DESKTOPABSOLUTEPARSING).ok()?;
        let parsing_name_text = parsing_name.to_string();
        windows::Win32::System::Com::CoTaskMemFree(Some(parsing_name.as_ptr() as *const _));

        crate::desktop::resolve_shell_item_path_from_parsing_name(&parsing_name_text.ok()?)
    }

    fn enumerate_child_windows(parent: HWND, child_windows: &mut Vec<HWND>) {
        let closure_pointer = child_windows as *mut Vec<HWND> as isize;
        unsafe extern "system" fn enumerate_callback(
            hwnd: HWND,
            lparam: LPARAM,
        ) -> windows::core::BOOL {
            let child_windows = &mut *(lparam.0 as *mut Vec<HWND>);
            child_windows.push(hwnd);

            true.into()
        }

        let _ = unsafe {
            EnumChildWindows(
                Some(parent),
                Some(enumerate_callback),
                LPARAM(closure_pointer),
            )
        };
    }

    fn client_point_from_screen(hwnd: HWND, point: &POINTL) -> POINT {
        let mut client_point = POINT {
            x: point.x,
            y: point.y,
        };
        let _ = unsafe { ScreenToClient(hwnd, &mut client_point) };

        client_point
    }
}

#[cfg(target_os = "windows")]
pub use windows_drop::{register_box_native_drop, unregister_box_native_drop};

/// 非 Windows 平台没有 Explorer Shell IDList 拖放，命令保持幂等成功以简化前端生命周期
#[cfg(not(target_os = "windows"))]
pub fn register_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}

/// 非 Windows 平台没有 Explorer Shell IDList 拖放，命令保持幂等成功以简化前端生命周期
#[cfg(not(target_os = "windows"))]
pub fn unregister_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}
