# Dasktop Fences 模式：原生桌面接管大版本计划

## Summary
把 Dasktop 从“Box 独立窗口 + 可选隐藏全部原生图标”升级为真正的 Fences 路线：运行时隐藏 Explorer 的 `SysListView32` 桌面图标层，由同进程 Rust/Win32 桌面控制器原生绘制未收纳图标；Box 继续由现有 Tauri/Vue 窗口承载。拖入 Box 只改映射，不移动真实文件；拖出 Box 后恢复到自绘桌面层的落点位置。

## Key Changes
- 新增同进程原生桌面控制器线程，不做 sidecar EXE，不在 Vue/WebView 里绘制桌面图标。
- 用 Win32 窗口/message loop 创建桌面绘制层，挂到桌面 WorkerW/Progman 层级；启动时隐藏 Explorer ListView，退出和失败时恢复显示。
- 不做跨进程 Explorer 子类化/注入；单项消失通过“隐藏原生整层 + Dasktop 自绘未收纳项”实现。
- 复用现有桌面扫描、Shell 图标、`IContextMenu` 能力；新增原生图标缓存、桌面坐标、框选、多选、拖拽、键盘操作和背景右键菜单。
- 旧的 `nativeDesktopIconsHidden` 语义替换为“桌面接管模式”，当前项目未上线，不保留旧配置兼容分支。

## Interfaces And Data Flow
- 新增 Rust 命令：
  - `set_desktop_takeover_enabled(enabled)`
  - `sync_desktop_takeover_state(state)`
  - `sync_desktop_takeover_box_bounds(bounds)`
  - `refresh_native_desktop_layer()`
- 新增运行时数据：
  - `DesktopTakeoverState`: 桌面项目、已收纳路径集合、Box 物理屏幕边界、图标位置、显示设置。
  - `DesktopItemPosition`: `itemPath/shellKey + monitorId + physical x/y`。
- Vue/Pinia/SQLite 仍是持久化源：Box 映射、Box 布局和桌面图标位置由前端保存；Rust 控制器只持有运行时副本。
- Rust 控制器通过 Tauri event 通知前端：
  - 桌面图标拖入 Box：前端写入 `box_items`，再同步新状态。
  - Box 图标拖回桌面：前端删除映射并保存落点坐标。
  - 桌面图标移动、重命名、刷新请求：前端持久化或触发重新扫描。
- Box 内拖拽逻辑需要携带释放屏幕坐标，替换现在“无人接收就删除映射但不知道落点”的行为。

## Desktop Behavior
- 第一版大版本目标包含：显示、双击打开、原生右键菜单、桌面背景右键、框选、多选、拖入/拖出 Box、图标位置保存、刷新、Delete、F2 重命名、基础复制/粘贴。
- 对 Shell 虚拟项使用现有 `shell::` 稳定键；不能被系统支持的操作禁用或交给原生菜单处理。
- 启动时尽量读取 Explorer 当前图标坐标；读取失败时使用网格布局，之后由 Dasktop 保存位置。
- Explorer 重启、显示器变化、DPI 变化时重新定位桌面层并重新应用隐藏/绘制状态。

## Test Plan
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `pnpm build`
- 手动验证：启动接管后 Explorer 原生图标不可见，自绘桌面图标可见。
- 拖桌面图标进 Box：桌面消失，Box 出现，真实文件路径不变。
- 拖 Box 图标回桌面：Box 消失，桌面落点出现，重启后保持。
- 验证双击打开、原生右键、框选、多选、重命名、删除、刷新。
- 验证 Explorer 重启、应用退出、异常启动恢复、多显示器和高 DPI。

## Assumptions
- 目标平台仍只支持 Windows。
- 默认启用桌面接管模式；如果控制器启动失败，必须恢复 Explorer 图标并向设置页报告错误。
- 真实文件不因收纳而移动、改名或删除；删除/重命名只在用户通过桌面操作明确触发时发生。
- 主 WebView 常驻隐藏运行，用来保存状态并接收原生桌面事件。
