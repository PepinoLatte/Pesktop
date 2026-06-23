//! Windows OLE DropTarget 封装，补齐透明 WebView 文件拖入不稳定的问题。

#[cfg(target_os = "windows")]
mod windows_drop {
    use crate::infrastructure::windows::shell_virtual_item;
    use serde::Serialize;
    use std::cell::{RefCell, UnsafeCell};
    use std::collections::HashMap;
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use std::sync::Arc;
    use tauri::{AppHandle, Emitter, EventTarget, Manager};
    use windows::core::{implement, Ref};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, POINTL};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::System::Com::{
        CoTaskMemFree, IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL,
    };
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
        DragQueryFileW, ILCombine, ILFree, IShellItemArray, SHCreateShellItemArrayFromDataObject,
        SHGetNameFromIDList, CFSTR_SHELLIDLIST, HDROP, SIGDN_DESKTOPABSOLUTEPARSING,
        SIGDN_FILESYSPATH,
    };
    use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;

    const DRAG_ENTER_EVENT: &str = "tauri://drag-enter";
    const DRAG_OVER_EVENT: &str = "tauri://drag-over";
    const DRAG_DROP_EVENT: &str = "tauri://drag-drop";
    const DRAG_LEAVE_EVENT: &str = "tauri://drag-leave";

    thread_local! {
        /// OLE DropTarget 必须在窗口线程注册和释放，因此控制器保存在 UI 线程本地注册表中。
        static NATIVE_DROP_CONTROLLERS: RefCell<HashMap<String, NativeDropController>> =
            RefCell::new(HashMap::new());
    }

    /// Drag/drop 事件载荷保持和 Tauri 内置 `onDragDropEvent` 同形，前端无需额外分支。
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeDragPayload {
        #[serde(skip_serializing_if = "Option::is_none")]
        paths: Option<Vec<String>>,
        position: NativeDropPosition,
        #[serde(skip_serializing_if = "Option::is_none")]
        shell_items: Option<Vec<NativeShellDropItem>>,
    }

    /// Shell 虚拟拖放项只暴露 Dasktop 支持的稳定 ID，避免前端保存系统 PIDL 细节。
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeShellDropItem {
        shell_id: String,
    }

    /// Tauri 前端会把 position 包成 PhysicalPosition，因此这里保留 x/y 数值结构。
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeDropPosition {
        x: f64,
        y: f64,
    }

    /// DropTarget 注册结果保存每个 WebView 子 HWND 的 COM 对象，避免被 Rust 提前释放。
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

    /// 为指定 Box 窗口注册原生拖放目标；重复注册会替换旧目标。
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

    /// 前端卸载和窗口销毁都会调用注销；未注册时保持幂等。
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

    /// 事件发送只依赖 app handle 和窗口 label，避免 DropTarget 持有前端对象引用。
    struct NativeDropEmitter {
        app: AppHandle,
        window_label: String,
    }

    impl NativeDropEmitter {
        fn emit_enter(&self, payload: NativeDropItems, position: POINT) {
            let paths = resolve_payload_paths(&payload);
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_ENTER_EVENT,
                NativeDragPayload {
                    paths: optional_non_empty(paths),
                    position: NativeDropPosition::from_point(position),
                    shell_items: optional_non_empty_shell_items(payload.shell_items),
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
                    shell_items: None,
                },
            );
        }

        fn emit_drop(&self, payload: NativeDropItems, position: POINT) {
            let paths = resolve_payload_paths(&payload);
            let _ = self.app.emit_to(
                EventTarget::labeled(&self.window_label),
                DRAG_DROP_EVENT,
                NativeDragPayload {
                    paths: optional_non_empty(paths),
                    position: NativeDropPosition::from_point(position),
                    shell_items: optional_non_empty_shell_items(payload.shell_items),
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

    /// 原生拖放解析结果同时承载真实文件路径和受支持的 Shell 虚拟桌面项。
    #[derive(Default)]
    struct NativeDropItems {
        paths: Vec<String>,
        shell_items: Vec<NativeShellDropItem>,
    }

    impl NativeDropItems {
        fn is_empty(&self) -> bool {
            self.paths.is_empty() && self.shell_items.is_empty()
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

    /// DropTarget 只接收真实路径或已知 Shell 虚拟项，避免不支持的数据源显示可投放光标。
    fn supports_supported_drop_data(data_obj: &IDataObject) -> bool {
        supports_hdrop_data(data_obj)
            || supports_shell_idlist_data(data_obj)
            || resolve_shell_drop_items(data_obj).is_some()
    }

    /// DragEnter 阶段部分 Explorer 数据源只承诺格式可用，真正内容到 Drop 阶段才稳定返回。
    fn supports_shell_idlist_data(data_obj: &IDataObject) -> bool {
        let Some(clipboard_format) = shell_idlist_clipboard_format() else {
            return false;
        };
        let format = FORMATETC {
            cfFormat: clipboard_format,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };

        unsafe { data_obj.QueryGetData(&format).is_ok() }
    }

    fn resolve_native_drop_items(data_obj: &IDataObject) -> Option<NativeDropItems> {
        let mut shell_items = resolve_shell_drop_items(data_obj).unwrap_or_default();
        for shell_item in resolve_shell_idlist_drop_items(data_obj).unwrap_or_default() {
            push_unique_shell_drop_item(&mut shell_items, &shell_item.shell_id);
        }
        let paths = resolve_hdrop_paths(data_obj)
            .unwrap_or_default()
            .into_iter()
            .filter(|path| {
                if let Some(shell_id) = shell_virtual_item::resolve_shell_id_from_parsing_name(path)
                {
                    push_unique_shell_drop_item(&mut shell_items, shell_id);
                    return false;
                }

                true
            })
            .collect();
        let drop_items = NativeDropItems { paths, shell_items };

        (!drop_items.is_empty()).then_some(drop_items)
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

    /// Windows 自带桌面图标通常没有 CF_HDROP，需从 Shell Item Array 中读取解析名再匹配白名单。
    fn resolve_shell_drop_items(data_obj: &IDataObject) -> Option<Vec<NativeShellDropItem>> {
        let item_array: IShellItemArray =
            unsafe { SHCreateShellItemArrayFromDataObject(data_obj).ok()? };
        let item_count = unsafe { item_array.GetCount().ok()? };
        let mut shell_items = Vec::new();

        for index in 0..item_count {
            let Ok(shell_item) = (unsafe { item_array.GetItemAt(index) }) else {
                continue;
            };
            let parsing_names = [
                unsafe { shell_item_display_name(&shell_item, SIGDN_DESKTOPABSOLUTEPARSING) },
                unsafe { shell_item_display_name(&shell_item, SIGDN_FILESYSPATH) },
            ];
            let Some(shell_id) = parsing_names.iter().flatten().find_map(|parsing_name| {
                shell_virtual_item::resolve_shell_id_from_parsing_name(parsing_name)
            }) else {
                continue;
            };

            push_unique_shell_drop_item(&mut shell_items, shell_id);
        }

        (!shell_items.is_empty()).then_some(shell_items)
    }

    /// Explorer 桌面系统图标常使用 `Shell IDList Array`，需要手动解析 CIDA 中的父 PIDL 和子 PIDL。
    fn resolve_shell_idlist_drop_items(data_obj: &IDataObject) -> Option<Vec<NativeShellDropItem>> {
        let clipboard_format = shell_idlist_clipboard_format()?;

        let format = FORMATETC {
            cfFormat: clipboard_format,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium = unsafe { data_obj.GetData(&format).ok()? };
        let hglobal = unsafe { medium.u.hGlobal };
        let locked = unsafe { GlobalLock(hglobal) };
        if locked.is_null() {
            unsafe {
                ReleaseStgMedium(&mut medium);
            }
            return None;
        }

        let size = unsafe { GlobalSize(hglobal) };
        let shell_items = unsafe { resolve_shell_idlist_items_from_memory(locked.cast(), size) };
        let _ = unsafe { GlobalUnlock(hglobal) };
        unsafe {
            ReleaseStgMedium(&mut medium);
        }

        shell_items
    }

    /// `CFSTR_SHELLIDLIST` 是注册剪贴板格式，集中转换成 `FORMATETC` 需要的 16 位编号。
    fn shell_idlist_clipboard_format() -> Option<u16> {
        let clipboard_format = unsafe { RegisterClipboardFormatW(CFSTR_SHELLIDLIST) };
        if clipboard_format == 0 || clipboard_format > u16::MAX as u32 {
            return None;
        }

        Some(clipboard_format as u16)
    }

    /// CIDA 内存布局为 cidl + (cidl + 1) 个 u32 偏移，第 0 个 PIDL 是父目录，后续是子项。
    unsafe fn resolve_shell_idlist_items_from_memory(
        base: *const u8,
        size: usize,
    ) -> Option<Vec<NativeShellDropItem>> {
        let minimum_header_size = std::mem::size_of::<u32>() * 2;
        if size < minimum_header_size {
            return None;
        }

        let item_count = std::ptr::read_unaligned(base.cast::<u32>()) as usize;
        let offset_count = item_count.checked_add(1)?;
        let header_size = std::mem::size_of::<u32>().checked_add(offset_count.checked_mul(4)?)?;
        if item_count == 0 || header_size > size {
            return None;
        }

        let mut offsets = Vec::with_capacity(offset_count);
        for index in 0..offset_count {
            let offset_pointer = base.add(std::mem::size_of::<u32>() + index * 4);
            let offset = std::ptr::read_unaligned(offset_pointer.cast::<u32>()) as usize;
            if offset >= size {
                return None;
            }
            offsets.push(offset);
        }

        let parent_pidl = base.add(offsets[0]).cast::<ITEMIDLIST>();
        let mut shell_items = Vec::new();
        for child_offset in offsets.iter().skip(1) {
            let child_pidl = base.add(*child_offset).cast::<ITEMIDLIST>();
            let absolute_pidl = ILCombine(Some(parent_pidl), Some(child_pidl));
            if absolute_pidl.is_null() {
                continue;
            }

            if let Some(parsing_name) = pidl_display_name(absolute_pidl) {
                if let Some(shell_id) =
                    shell_virtual_item::resolve_shell_id_from_parsing_name(&parsing_name)
                {
                    push_unique_shell_drop_item(&mut shell_items, shell_id);
                }
            }
            ILFree(Some(absolute_pidl));
        }

        (!shell_items.is_empty()).then_some(shell_items)
    }

    /// PIDL 名称由 Shell 分配，需要释放 PWSTR，避免拖放系统图标时泄漏内存。
    unsafe fn pidl_display_name(pidl: *const ITEMIDLIST) -> Option<String> {
        let display_name = SHGetNameFromIDList(pidl, SIGDN_DESKTOPABSOLUTEPARSING).ok()?;
        let value = display_name.to_string().ok();
        CoTaskMemFree(Some(display_name.0 as *const _));

        value
    }

    /// 一个拖放数据对象可能同时提供 Shell Item 和 HDROP，按 shell_id 去重避免前端重复保存。
    fn push_unique_shell_drop_item(shell_items: &mut Vec<NativeShellDropItem>, shell_id: &str) {
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

    /// Shell 分配的 PWSTR 由调用方释放，避免持续拖放系统图标时泄漏 COM 内存。
    unsafe fn shell_item_display_name(
        shell_item: &windows::Win32::UI::Shell::IShellItem,
        sigdn: windows::Win32::UI::Shell::SIGDN,
    ) -> Option<String> {
        let display_name = shell_item.GetDisplayName(sigdn).ok()?;
        let value = display_name.to_string().ok();
        CoTaskMemFree(Some(display_name.0 as *const _));

        value
    }

    fn optional_non_empty(values: Vec<String>) -> Option<Vec<String>> {
        (!values.is_empty()).then_some(values)
    }

    /// Tauri 前端 `onDragDropEvent` 会规整 payload 并丢弃未知字段，因此 Shell 项也必须写入标准 paths。
    fn resolve_payload_paths(payload: &NativeDropItems) -> Vec<String> {
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

    fn optional_non_empty_shell_items(
        values: Vec<NativeShellDropItem>,
    ) -> Option<Vec<NativeShellDropItem>> {
        (!values.is_empty()).then_some(values)
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

/// 非 Windows 平台没有 Explorer OLE 拖放目标，命令保持幂等成功以简化前端生命周期。
#[cfg(not(target_os = "windows"))]
pub fn register_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}

/// 非 Windows 平台没有 Explorer OLE 拖放目标，命令保持幂等成功以简化前端生命周期。
#[cfg(not(target_os = "windows"))]
pub fn unregister_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}
