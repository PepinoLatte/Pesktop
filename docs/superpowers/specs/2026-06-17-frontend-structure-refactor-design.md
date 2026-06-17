# 前端目录结构重构设计

日期：2026-06-17
状态：已批准，待实现

## 背景

当前前端为 feature-based 结构（`src/features/` + `src/shared/`），底子可用，但依赖图扫描后发现三类问题：

1. **`shared/` 在膨胀成杂物间**：已有 `api / components / config / events / storage / types / window` 七个子目录，职责混杂。
2. **领域模型被错放进 `shared`**：`shared/types/desktop.ts`、`shared/config/desktopLayout.ts`、`shared/config/appSettings.ts` 被 desktop + settings 双方引用，本质是领域模型而非通用工具。
3. **feature 间已存在反向依赖**：`features/settings/index.vue` 直接 import 了 `features/desktop/store/desktopStore`，说明 store 需提到中性层。
4. **独立 Tauri webview 被埋成子组件**：`BoxContextMenuWindow.vue`、`DragPreviewWindow.vue` 是独立运行的 webview 实例，却放在 `features/desktop/components/` 下。

## 目标

按「独立窗口 / 领域模型 / 通用工具」三层重新组织前端，并将仅单一组件消费的纯展示数据/配置内联回组件，减少无意义的文件跳转。

## 关键决策

1. **引入 `@/` 路径别名**：在 `vite.config.ts` 与 `tsconfig.json` 配置 `@/*` → `src/*`，所有跨目录 import 一律走 `@/...`，避免深层相对路径脆弱。
2. **`App.vue` 窗口分发逻辑不变**：仅更新 4 个窗口组件的 import 路径，不改为懒加载（留作独立任务）。
3. **一次性完成全部改动**：搬迁 + 改 import + 内联一并执行，用 `vue-tsc --noEmit` 检查无语法/类型错误即可，不跑完整 `vite build`。

## 目标结构

```
src/
├── app/
│   ├── App.vue                # 按 windowLabel 分发挂载哪个窗口（逻辑不变）
│   ├── main.ts
│   └── styles.css             # 已在此处
│
├── windows/                   # 每个独立 Tauri webview = 一个顶层模块
│   ├── desktop/
│   │   ├── index.vue
│   │   ├── components/
│   │   │   ├── DesktopIcon.vue
│   │   │   └── DesktopIconGlyph.vue   # WINDOWS_SHORTCUT_BADGE 内联进此组件
│   │   └── config/
│   │       └── desktopIcon.ts         # 仅保留 DESKTOP_ICON_VIEW（4 处共享）
│   │
│   ├── settings/
│   │   ├── index.vue                  # SettingsSection/NavItem 类型 + SETTINGS_PANEL_WIDTH 内聚于此
│   │   └── components/
│   │       ├── SettingsHeader.vue
│   │       ├── SettingsSidebar.vue
│   │       ├── AboutPanel.vue
│   │       ├── BoxesPanel.vue
│   │       ├── AppearancePanel.vue    # THEME_SEGMENT_OPTIONS 内联
│   │       └── BoxDisplayPanel.vue    # NAME_DISPLAY / BOX_VISUAL_CONTROLS / SNAP_INPUT / BoxVisualSettingKey 内联
│   │
│   ├── boxContextMenu/
│   │   └── index.vue                  # 原 BoxContextMenuWindow.vue
│   │
│   └── dragPreview/
│       ├── index.vue                  # 原 DragPreviewWindow.vue
│       └── lifecycle.ts               # 原 shared/window/dragPreviewWindow.ts
│
├── entities/                          # 跨窗口共享的领域模型
│   ├── desktopItem/
│   │   ├── types.ts                   # DesktopItem, DesktopNameDisplayMode
│   │   ├── api.ts                     # 原 shared/api/desktop.ts（6 个 invoke 封装）
│   │   └── displayName.ts             # formatDesktopItemDisplayName
│   ├── desktopBox/
│   │   ├── types.ts                   # DesktopBox, DesktopBoxTitlePosition, DesktopSnapshot
│   │   ├── layout.ts                  # 原 shared/config/desktopLayout.ts
│   │   ├── store.ts                   # 原 features/desktop/store/desktopStore.ts
│   │   └── windows.ts                 # 原 shared/window/boxWindows.ts
│   └── appSettings/
│       ├── types.ts                   # AppSettings, ThemeMode
│       └── defaults.ts                # 原 shared/config/appSettings.ts（含 DEFAULT_APP_SETTINGS, APP_SETTING_NUMBER_LIMITS）
│
├── shared/                            # 真·跨业务，无领域知识
│   ├── ui/
│   │   └── SegmentedControl.vue
│   ├── ipc/                           # 窗口间事件契约（原 shared/events/）
│   │   ├── boxContextMenu.ts
│   │   ├── boxItemDrag.ts
│   │   └── desktop.ts
│   └── storage/
│       └── database.ts
│
└── assets/
```

## 搬迁映射

| 原位置 | 新位置 | 理由 |
|---|---|---|
| `src/App.vue` | `src/app/App.vue` | 入口集中到 app/ |
| `src/main.ts` | `src/app/main.ts` | 同上；需同步改 index.html 的 script src 与 main.ts 内 import |
| `features/desktop/index.vue` | `windows/desktop/index.vue` | 桌面主窗口 |
| `features/desktop/components/DesktopIcon.vue` | `windows/desktop/components/DesktopIcon.vue` | |
| `features/desktop/components/DesktopIconGlyph.vue` | `windows/desktop/components/DesktopIconGlyph.vue` | |
| `features/desktop/config/desktopIcon.ts` | `windows/desktop/config/desktopIcon.ts` | 仅保留 DESKTOP_ICON_VIEW |
| `features/desktop/store/desktopStore.ts` | `entities/desktopBox/store.ts` | settings 反向依赖它 |
| `features/desktop/utils/desktopItemName.ts` | `entities/desktopItem/displayName.ts` | 双消费者纯函数 |
| `features/desktop/components/BoxContextMenuWindow.vue` | `windows/boxContextMenu/index.vue` | 独立 webview |
| `features/desktop/components/DragPreviewWindow.vue` | `windows/dragPreview/index.vue` | 独立 webview |
| `features/settings/index.vue` | `windows/settings/index.vue` | 设置窗口 |
| `features/settings/components/*` | `windows/settings/components/*` | |
| `shared/types/desktop.ts` | 拆到 `entities/{desktopItem,desktopBox,appSettings}/types.ts` | 单文件混了三个领域 |
| `shared/config/desktopLayout.ts` | `entities/desktopBox/layout.ts` | Box 领域配置 |
| `shared/config/appSettings.ts` | `entities/appSettings/defaults.ts` | AppSettings 领域默认值 |
| `shared/window/boxWindows.ts` | `entities/desktopBox/windows.ts` | Box 窗口操作 |
| `shared/api/desktop.ts` | `entities/desktopItem/api.ts` | Item 能力，IPC 边界 |
| `shared/window/dragPreviewWindow.ts` | `windows/dragPreview/lifecycle.ts` | 与 dragPreview 组件两端关系 |
| `shared/events/boxContextMenuEvents.ts` | `shared/ipc/boxContextMenu.ts` | 窗口间 IPC 契约 |
| `shared/events/boxItemDragEvents.ts` | `shared/ipc/boxItemDrag.ts` | 同上 |
| `shared/events/desktopEvents.ts` | `shared/ipc/desktop.ts` | 同上 |
| `shared/components/SegmentedControl.vue` | `shared/ui/SegmentedControl.vue` | 通用 UI |
| `shared/storage/database.ts` | `shared/storage/database.ts` | 位置不变 |

## 组件内联

删除 2 个文件 + 1 个文件瘦身。准则：**单一消费者 + 纯展示数据 → 内联；多消费者 / IPC / 窗口边界 / 纯逻辑函数 → 保持独立。**

| 源 | 去向 | 处理 |
|---|---|---|
| `settings/types.ts`（SettingsSection, SettingsNavItem） | `windows/settings/index.vue` | 内联，删文件；SettingsSidebar 改就近 import 或 props 推导 |
| `settingsUi.ts` THEME_SEGMENT_OPTIONS | `AppearancePanel.vue` | 内联 |
| `settingsUi.ts` NAME_DISPLAY_SEGMENT_OPTIONS / BOX_VISUAL_SETTING_CONTROLS / SNAP_THRESHOLD_INPUT / BoxVisualSettingKey | `BoxDisplayPanel.vue` | 内联；BoxVisualSettingKey 依赖的 AppSettingNumberKey 改 import 自 `@/entities/appSettings/defaults` |
| `settingsUi.ts` SETTINGS_PANEL_WIDTH | `windows/settings/index.vue` | 内联（3 个 panel 共用，提到父级），settingsUi.ts 删文件 |
| `desktopIcon.ts` WINDOWS_SHORTCUT_BADGE | `DesktopIconGlyph.vue` | 内联，desktopIcon.ts 只留 DESKTOP_ICON_VIEW |

### 保持独立（不内联）

- `DESKTOP_ICON_VIEW`：被 index.vue / DesktopIcon / DesktopIconGlyph / DragPreviewWindow 4 处引用。
- `entities/desktopItem/api.ts`：6 个 invoke 封装，IPC 边界。
- `windows/dragPreview/lifecycle.ts`：窗口创建/复用/ready 握手逻辑重，与组件两端关系。
- `entities/desktopItem/displayName.ts`：DesktopIcon + DragPreviewWindow 双消费者纯函数。

## 实现注意点

1. **`@/` 别名两处都要配**：`vite.config.ts` 的 `resolve.alias` 与 `tsconfig.json` 的 `compilerOptions.paths`（配 `baseUrl`/`paths`）。
2. **入口移动**：`App.vue`/`main.ts` 移入 `app/` 后，`index.html` 的 `/src/main.ts` 改为 `/src/app/main.ts`，`main.ts` 内 `./App.vue` 与 `./app/styles.css` 路径同步修正。
3. **`shared/types/desktop.ts` 拆分**：拆成三个 entities types 文件后，所有引用按符号归属改 import 来源（DesktopItem/DesktopNameDisplayMode → desktopItem；DesktopBox/DesktopBoxTitlePosition/DesktopSnapshot → desktopBox；AppSettings/ThemeMode → appSettings）。
4. **`dragPreviewWindow.ts` 拆分位置**：组件侧用到的 `DRAG_PREVIEW_WINDOW_READY_EVENT` 与窗口创建逻辑同放 `lifecycle.ts`，DragPreviewWindow.vue 从 `./lifecycle` import。

## 验证

- 全部改动完成后运行 `pnpm exec vue-tsc --noEmit`，确认无语法与类型错误。
- 不运行完整 `vite build`。
- Tauri/Rust 侧（src-tauri）不在本次范围内。

## 不做的事（YAGNI / 范围外）

- 不改 `App.vue` 窗口分发为懒加载。
- 不引入 `features/` 中间层（当前每个功能≈一个窗口）。
- 不重构 Rust 侧。
- 不新增测试框架。
