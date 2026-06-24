//! 前后端共享应用契约常量集中在领域层，避免命令层散落魔法字符串。

/// 设置窗口 label 由 Tauri 配置和前端入口共同约定，托盘与单实例唤起都依赖它。
pub const SETTINGS_WINDOW_LABEL: &str = "main";

/// 托盘请求创建 Box 的前端事件；主 WebView 作为隐藏控制器复用现有 Store 创建流程。
pub const TRAY_CREATE_BOX_EVENT: &str = "dasktop://tray-create-box";

/// 自启状态变化事件用于同步隐藏设置窗里的开关状态。
pub const AUTOSTART_CHANGED_EVENT: &str = "dasktop://autostart-changed";

/// Tauri 拖拽进入事件名；原生 DropTarget 使用同名事件保持前端处理链路一致。
pub const TAURI_DRAG_ENTER_EVENT: &str = "tauri://drag-enter";

/// Tauri 拖拽移动事件名；原生 DropTarget 使用同名事件保持前端处理链路一致。
pub const TAURI_DRAG_OVER_EVENT: &str = "tauri://drag-over";

/// Tauri 拖拽释放事件名；原生 DropTarget 使用同名事件保持前端处理链路一致。
pub const TAURI_DRAG_DROP_EVENT: &str = "tauri://drag-drop";

/// Tauri 拖拽离开事件名；原生 DropTarget 使用同名事件保持前端处理链路一致。
pub const TAURI_DRAG_LEAVE_EVENT: &str = "tauri://drag-leave";
