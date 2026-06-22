# Box 相关窗口结构拆分计划

## Summary

参考 `windows/settings` 的拆分方式，但不过度套模板：三个窗口都保留 `index.vue` 作为壳组件，将状态、生命周期、业务动作和纯 UI 分离。审核通过后，先把本计划复制到 `docs/frontend-box-windows-structure-refactor-plan.md`，再开始改动。

## Target Structure

```text
src/windows/
├── boxContextMenu/
│   ├── index.vue
│   ├── model/
│   │   └── menuOptions.ts
│   ├── composables/
│   │   ├── useBoxContextMenuWindow.ts
│   │   └── useBoxContextMenuActions.ts
│   └── components/
│       ├── MenuTopActions.vue
│       ├── MenuSettingControls.vue
│       ├── MenuActionRows.vue
│       └── MenuDeleteButton.vue
│
├── desktop/
│   ├── index.vue
│   ├── model/
│   │   ├── fileDrag.ts
│   │   └── selection.ts
│   ├── components/
│   │   ├── BoxHeader.vue
│   │   ├── BoxResizeHandles.vue
│   │   └── BoxFileGrid.vue
│   ├── composables/
│   │   ├── useBoxFileItems.ts
│   │   ├── useBoxFileSelection.ts
│   │   ├── useBoxFileDrag.ts
│   │   ├── useBoxFileActions.ts
│   │   └── useBoxWindowLifecycle.ts
│   ├── config/
│   └── utils/
│
└── dragPreview/
    ├── index.vue
    ├── lifecycle.ts
    ├── model/
    │   └── previewLayout.ts
    ├── composables/
    │   └── useDragPreviewWindow.ts
    └── components/
        └── DragPreviewItem.vue
```

## Key Changes

- `boxContextMenu/index.vue` 收缩为菜单窗口壳：初始化 Store、挂载窗口生命周期 composable、渲染菜单组件。菜单选项、分段控件配置和按钮 class 迁到 `model/menuOptions.ts`；打开/关闭动画与 IPC 监听迁到 `useBoxContextMenuWindow.ts`；新建、刷新、打开文件夹、锁定、透明度、删除等动作迁到 `useBoxContextMenuActions.ts`。
- `desktop/index.vue` 只保留 Box 窗口布局编排。文件扫描/轮询放入 `useBoxFileItems.ts`，框选和键盘选择放入 `useBoxFileSelection.ts`，跨窗口文件拖拽放入 `useBoxFileDrag.ts`，打开/重命名/删除/右键菜单放入 `useBoxFileActions.ts`，Tauri 监听注册和卸载收口到 `useBoxWindowLifecycle.ts`。
- `desktop` 现有 `useBoxWindowFrame / useBoxCollapsePreview / useBoxContextMenu / useBoxTitleEditing` 保留，不重写核心行为；只调整调用边界，让新 composables 与它们协作。
- `dragPreview/index.vue` 轻拆即可：拖影显示组件迁到 `DragPreviewItem.vue`，尺寸计算常量和纯函数迁到 `model/previewLayout.ts`，事件监听、窗口移动、窗口 resize 迁到 `useDragPreviewWindow.ts`。
- 不移动 `shared/ipc/*`、`entities/desktopBox/*`、`entities/desktopItem/*`，它们仍是跨窗口契约和业务实体边界；不修改 `App.vue` 的 query 参数路由。

## Public API / Interface

- 前端路由参数保持不变：`boxId`、`boxMenu`、`dragPreview` 不改。
- 跨窗口 IPC 事件名和 payload 不改：`boxContextMenu.ts`、`boxFileDrag.ts` 只被复用，不迁移。
- 组件拆分只改变窗口内部文件结构，不改变 Rust invoke 命令名、不改变 Store 字段、不改变设置项结构。

## Test Plan

- 运行 `pnpm exec vue-tsc --noEmit`。
- 手测 Box 窗口：创建/打开 Box、标题编辑、移动、缩放、边缘吸附、自动收起、锁定状态。
- 手测文件区：刷新列表、单选、多选、框选、Ctrl/Meta 多选、F2 重命名、Delete 删除、Enter 打开、右键原生菜单。
- 手测拖拽：Box 内拖动、多选拖动、拖到另一个 Box、拖出到桌面、拖拽取消、拖影窗口显示/隐藏/计数。
- 手测 Box 更多菜单：打开/关闭动画、失焦关闭、设置入口、新增、刷新、标题位置、自动收起、锁定、闲置可见度、删除二次确认。

## Assumptions

- 本次是结构重构，不改变功能行为和视觉样式。
- 旧文件拆空后直接删除，不保留 re-export 兼容入口。
- 所有新增 Vue/TypeScript 文件保持 UTF-8，并补充 JSDoc/业务意图注释。
- 审核通过后第一步先创建 `docs/frontend-box-windows-structure-refactor-plan.md`，再开始代码拆分。
