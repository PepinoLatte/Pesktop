# 发行版右键菜单与卸载提示文案设计

## 背景

Dasktop 的窗口由 Tauri WebView 承载。开发环境中，WebView 原生右键菜单对调试很有用；发行版中，它会暴露浏览器式菜单和调试痕迹，破坏桌面工具体验。项目已经对 Box 文件项右键做了专门处理，会调用 Windows Shell 原生文件菜单，因此全局隐藏 WebView 默认菜单时必须保留事件传播。

NSIS 卸载器目前把删除应用数据描述为“不会删除真实桌面文件”。随着 Dasktop 会创建 Box 文件夹，用户可能把文件放进这些文件夹；如果卸载时选择删除配置和应用数据，需要明确提醒这些文件夹及其中内容可能被删除，避免用户误删文件。

## 目标

- 发行版隐藏 WebView 原生右键菜单。
- 开发环境保留 WebView 原生右键菜单，方便调试。
- 不影响 Box 文件项右键触发 Windows Shell 原生菜单。
- 调整 NSIS 中文卸载文案，说明删除配置和应用数据时可能删除 Dasktop 创建的 Box 文件夹及其中内容，请用户提前转移需要保留的文件。

## 非目标

- 不改变 Box 文件项 Windows Shell 右键菜单。
- 不修改 WiX 文案，因为当前删除应用数据选项来自 NSIS 中文语言文件。
- 不增加旧字段、旧配置或迁移分支。

## 方案

在 `src/app/main.ts` 中注册生产环境专用的 `contextmenu` 捕获阶段监听器，只调用 `preventDefault()`。捕获阶段可以覆盖所有 WebView 页面；不调用 `stopPropagation()`，确保组件级右键逻辑仍能收到事件并执行。

在 `src-tauri/bundle/nsis/languages/SimpChinese.nsh` 中更新文件头注释和 `deleteAppData` 文案。文案直接使用“删除 Dasktop 配置和应用数据”，并说明软件创建的 Box 文件夹及其中内容可能被删除。

## 测试策略

- 运行 `pnpm build`，验证 TypeScript 和 Vite 构建通过。
- 人工验证发行版右键：普通页面空白区域不弹 WebView 原生菜单。
- 人工验证开发环境右键：`pnpm dev` 下保留 WebView 原生菜单。
- 人工验证 Box 文件项右键：仍弹出 Windows Shell 原生菜单。
