# Dasktop 桌面扩展软件技术选型计划

## Summary

Dasktop V1 采用 `Tauri 2 + Vue 3 + SQLite`。这套技术栈保留 Web UI 的开发效率，同时用 Rust 承担 Windows 桌面目录扫描和系统能力，比 Electron 更轻，也比直接选择 WPF/WinUI 更利于后续跨平台探索。

V1 是桌面扩展软件，不是桌面管理页，也不覆盖或替换系统桌面。产品形态调整为“主控制窗 + 多 Box 独立窗”：保留 Explorer 原生桌面、右键菜单和图标拖动能力；每个 Box 是一个独立透明小窗口，Box 之外不放任何全屏透明背景。真实桌面文件不会被移动、改名或删除，所有整理行为只写入 Box 映射关系。

## 技术栈与版本

- 前端：`Vue 3.5.38`、`Pinia 3.0.4`、`Tailwind CSS 4.3.1`、`@tailwindcss/vite 4.3.1`、`@tauri-apps/api 2.11.0`、`@tauri-apps/plugin-sql 2.4.0`。
- 构建：`Vite 8.0.16`、`@vitejs/plugin-vue 6.0.7`、`TypeScript 6.0.3`、`vue-tsc 3.3.5`、`@tauri-apps/cli 2.11.2`；Node 要求 `>=20.19.0`。
- Rust/Tauri：`tauri 2.11.2`、`tauri-plugin-sql 2.4.0` with `sqlite` feature。
- 不引入 Axum/Actix。桌面本地能力通过 Tauri commands 和官方插件完成，避免为了本地进程通信额外维护 HTTP 服务。
- 所有前端依赖使用 exact version，锁定 `pnpm-lock.yaml`；Rust 依赖锁定 `Cargo.lock`。

## V1 功能

- 桌面增量：默认不隐藏 Explorer 桌面图标，不拦截系统桌面右键。
- Box 窗口：每个 Box 由独立 Tauri `WebviewWindow` 承载，透明、无边框、可拖动、可调整尺寸；窗口外区域完全交还 Windows 桌面。
- 图标系统：扫描用户桌面目录，识别文件、文件夹和快捷方式，返回给前端渲染。
- 分组交互：创建 Box、移动/缩放 Box、从 Windows 桌面或 Explorer 原生拖入文件到 Box，并把映射写入 SQLite。
- 设置窗口：由 `main` 窗口承载，启动时恢复 Box 后隐藏；Box 右键菜单可重新打开设置。设置窗采用不透明左右结构，提供浅色、暗色、跟随系统主题，以及边缘吸附开关和吸附阈值。
- 边缘吸附：Box 窗口移动到当前显示器工作区边缘附近时，吸附到屏幕边缘；窗口移动和尺寸变化会写回 Box 映射。
- UI 风格：Apple + Raycast 风；设置窗口使用不透明系统面板，Box 使用透明圆角容器，不使用渐变背景；图标统一使用 Lucide，支持浅色、暗色、跟随系统和 reduced-motion 保护。

## 项目结构

```text
src/
  app/                 # Vue app bootstrap and Tailwind styles
  features/desktop/    # Box windows, icon grid, Pinia store
  features/settings/   # 设置窗口与后续偏好配置
  shared/              # API wrapper, SQLite storage, shared types

src-tauri/src/
  commands/            # Tauri command boundary
  desktop/             # Desktop path scan and item mapping
```

## 数据与接口

SQLite 表：

- `boxes`：Box 的标题、位置、尺寸和 `item_paths` 映射。
- `app_settings`：后续保存开机启动、显示偏好等轻量设置。

Tauri commands：

- `get_desktop_snapshot()`

Box 布局、文件映射和设置全部由前端 SQLite 插件持久化；Rust 侧不保留旧的 Box 占位命令。

## 注释规范

代码注释遵循阿里巴巴注释风格：公共类型、公共函数、复杂业务逻辑必须解释用途和原因；避免无意义翻译代码；TODO 必须带明确原因和后续动作。TypeScript 使用 JSDoc，Rust 使用 rustdoc。

## 验收

- `pnpm install --frozen-lockfile`
- `pnpm build`
- `cargo check`
- 手动验证：启动后能扫描桌面文件；创建、移动、缩放 Box；从 Windows 桌面或 Explorer 拖入桌面文件后重启保持映射；桌面空白区域仍使用 Windows 原生右键。
