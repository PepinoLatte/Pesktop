//! 前端热更新支持：外置前端资源检测、自定义协议伺服与全窗口重载。
//!
//! 生产构建默认把前端资源内嵌进二进制；热更流程把最新 `dist` 放到可执行文件旁，
//! 启动检测到 `dist/index.html` 时所有窗口改走 `http://daskhot.localhost/` 自定义协议
//! 从磁盘实时读取，配合托盘「重载界面」或二次启动 `--reload` 即可不重编译验证新前端。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Url};

/// 自定义协议名；Windows 侧访问地址形如 `http://daskhot.localhost/index.html`。
pub(crate) const EXTERNAL_FRONTEND_SCHEME: &str = "daskhot";

/// 主窗口标签；外置模式下 setup 阶段需要把它从内嵌资源导航到 daskhot 协议。
const MAIN_WINDOW_LABEL: &str = "main";

/// 外置资源目录探测结果缓存；目录在运行期不会出现或消失，避免每次请求重复扫描文件系统。
static EXTERNAL_DIST_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// 返回外置前端资源目录（可执行文件旁的 `dist`）；不存在时返回 None，全部窗口走内嵌资源。
pub(crate) fn detect_external_dist_dir() -> Option<&'static PathBuf> {
    EXTERNAL_DIST_DIR
        .get_or_init(|| {
            let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
            let dist_dir = exe_dir.join("dist");
            dist_dir.join("index.html").is_file().then_some(dist_dir)
        })
        .as_ref()
}

/// 当前是否启用外置前端资源；该结果同时暴露给前端用于构造动态窗口 URL。
pub(crate) fn is_external_frontend_enabled() -> bool {
    detect_external_dist_dir().is_some()
}

/// 拼接外置模式下的窗口加载地址；解析失败仅在日志可见，错误类型沿用项目的 String 约定。
pub(crate) fn external_frontend_url(path: &str) -> Result<Url, String> {
    Url::parse(&format!("http://{EXTERNAL_FRONTEND_SCHEME}.localhost{path}"))
        .map_err(|error| format!("外置前端地址解析失败：{error}"))
}

/// setup 阶段把 main 窗口从内嵌资源导航到外置协议；内嵌模式下保持默认行为不动。
pub(crate) fn navigate_main_window_to_external_frontend(app: &tauri::App) {
    if !is_external_frontend_enabled() {
        return;
    }

    let Some(main_window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    match external_frontend_url("/index.html") {
        Ok(url) => {
            if let Err(error) = main_window.navigate(url) {
                eprintln!("failed to navigate main window to external frontend: {error}");
            }
        }
        Err(error) => eprintln!("failed to parse external frontend url: {error}"),
    }
}

/// `daskhot` 协议处理器：把 URL 路径映射到外置 dist 内的静态文件并回填 MIME。
pub(crate) fn handle_external_frontend_request(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(dist_dir) = detect_external_dist_dir() else {
        return not_found_response();
    };

    // 只取 path；窗口携带的 boxId 等 query 由前端路由消费，静态伺服不关心
    let relative_path = request.uri().path().trim_start_matches('/');
    // 静态伺服不允许逃出 dist 目录，含 .. 段的路径一律拒绝
    if relative_path.is_empty() || relative_path.split('/').any(|segment| segment == "..") {
        return not_found_response();
    }

    let mut file_path = dist_dir.join(relative_path);
    // SPA 兜底：无扩展名的路径统一回退 index.html，保证刷新或深链接不落 404
    if file_path.is_dir() || Path::new(relative_path).extension().is_none() {
        file_path = dist_dir.join("index.html");
    }

    match std::fs::read(&file_path) {
        Ok(contents) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, resolve_mime_type(&file_path))
            .body(contents)
            .unwrap_or_else(|_| not_found_response()),
        Err(_) => not_found_response(),
    }
}

/// 全窗口重载：热更后旧页面仍在内存里运行，逐 WebView 触发 location.reload 换新资源。
pub(crate) fn reload_all_webviews(app_handle: &AppHandle) {
    for (label, webview) in app_handle.webview_windows() {
        if let Err(error) = webview.eval("window.location.reload()") {
            eprintln!("failed to reload webview {label}: {error}");
        }
    }
}

/// 构建 404 空响应；Response::builder 对固定字段不会失败，直接 expect 暴露异常。
fn not_found_response() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Vec::new())
        .expect("静态 404 响应构建失败")
}

/// 按 Vite 产物可能出现的扩展名回填 MIME；未知类型按字节流处理。
fn resolve_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}
