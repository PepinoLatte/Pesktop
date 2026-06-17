# Dasktop 启动、托盘、自启与 Box 管理计划

## Summary
- 暂不处理第 4 点：不改“此电脑 / 回收站”等 Windows 原生特殊图标的拖入或隐藏策略。
- 其余功能按现有架构接入：主窗口继续作为隐藏控制器，Box 能力复用 `desktopStore` 和 `openBoxWindow/closeBoxWindow`，系统托盘使用 Tauri 官方 tray API，自启使用官方 autostart 插件。
- 当前工作区已有用户改动：`src/windows/desktop/composables/useBoxCollapsePreview.ts` 是脏文件，本次实现不触碰它。

## Key Changes
- **静默启动**
  - 在 `src-tauri/tauri.conf.json` 的 main window 增加 `visible: false`。
  - 保留设置页 `onMounted` 的初始化、打开 Box、隐藏自身逻辑，启动时不再先闪出设置窗。
  - 设置窗仍可通过 Box 更多菜单和托盘菜单重新显示。

- **系统托盘**
  - 给 `tauri` 启用 `tray-icon` feature，按 Tauri 官方 System Tray API 创建托盘图标和菜单。
  - 新增 Rust 侧托盘模块，例如 `src-tauri/src/app_tray.rs`，统一维护菜单项和事件：
    - `设置`：显示并聚焦 main 设置窗。
    - `新增 Box`：向 main webview 发送创建 Box 事件，由前端复用现有 `desktopStore.createBox()` + `openBoxWindow()`。
    - `开机自启`：复用 autostart 插件切换状态，并同步托盘勾选状态。
    - `关闭`：调用 `app.exit(0)`，继续走现有退出时恢复原生桌面图标逻辑。
  - 左键单击托盘图标打开设置；右键显示菜单，避免左键直接弹菜单。

- **开机自启**
  - 添加官方 `tauri-plugin-autostart` Rust 插件，不手写 Windows 注册表逻辑。
  - 新增 Rust commands：
    - `is_autostart_enabled() -> Result<bool, String>`
    - `set_autostart_enabled(enabled: bool) -> Result<bool, String>`
  - 新增前端 API wrapper，例如 `src/entities/appSettings/api.ts`，设置页通过 invoke 调用，不直接接触插件细节。
  - `BoxDisplayPanel` 的“隐藏系统桌面图标”同一个 card 中新增“开机自启”开关；自启状态以系统插件状态为准，不写入 SQLite 的 `app_settings`。

- **Box 设置页管理**
  - `BoxesPanel` 增加每个 Box 的操作按钮：打开、锁定/解锁、删除。
  - 避免嵌套 button：列表行改成容器布局，打开区域和图标按钮分离。
  - 锁定/解锁复用 `desktopStore.updateBoxLocked()`；删除复用 `desktopStore.deleteBox()` 和 `closeBoxWindow()`。

- **删除二次确认**
  - 新增一个小型复用确认 helper，例如 `src/entities/desktopBox/deleteConfirmation.ts`。
  - Box 更多菜单和设置页删除都先调用同一确认文案。
  - 文案明确：删除 Box 只移除分组窗口和映射，不删除真实文件。
  - 用户取消时不删除、不关闭对应 Box。

## Public Interfaces
- Tauri commands 新增：
  - `is_autostart_enabled`
  - `set_autostart_enabled`
- 前端事件新增：
  - `dasktop://tray-create-box`：Rust 托盘请求 main webview 创建并打开新 Box。
  - `dasktop://autostart-changed`：托盘或设置页切换后同步 UI 状态。
- `BoxDisplayPanel` props/emits 新增：
  - prop：`autostartEnabled: boolean`
  - emit：`autostartEnabledChange: [value: boolean]`
- `BoxesPanel` emits 新增：
  - `toggleBoxLocked: [box: DesktopBox]`
  - `deleteBox: [box: DesktopBox]`

## Test Plan
- 自动检查：
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `pnpm build`
- 手动验证：
  - 启动应用后设置窗不闪现，已有 Box 正常恢复。
  - 托盘图标出现；右键菜单含设置、新增 Box、开机自启、关闭。
  - 托盘“设置”能显示设置窗；“新增 Box”能创建并打开 Box，设置窗不必弹出。
  - 设置页“开机自启”开关和托盘菜单勾选状态互相同步。
  - Box 更多菜单删除、设置页删除均出现二次确认；取消不删除，确认后关闭对应 Box 窗口且真实文件不受影响。
  - 设置页锁定/解锁后，对应 Box 的移动和缩放能力与更多菜单一致。

## Assumptions
- 第 4 点完全不动：不修改 `native_icons.rs`，不改变 `nativeDesktopIconsHidden` 当前“隐藏整个 Explorer 原生桌面图标层”的语义。
- 使用官方能力避免重复造轮子：托盘参考 Tauri System Tray 文档，开机自启参考 Tauri Autostart 插件文档。
- 如果添加 `tauri-plugin-autostart` 需要联网，实施时应请求允许执行官方依赖安装命令，不改为手写注册表 fallback。

参考：
- https://v2.tauri.app/learn/system-tray/
- https://v2.tauri.app/plugin/autostart/
