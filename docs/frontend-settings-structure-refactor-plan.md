# 前端与设置弹窗结构拆分方案

## Summary

沿用当前已经形成的 `app / windows / entities / shared` 分层，不重新引入 `features`。本次重点拆 `windows/settings`：把设置窗口从 4 个菜单调整为 5 个菜单，收纳能力合并进 Box 页，并把 `index.vue` 里的窗口生命周期、Box 启动恢复、设置动作处理拆到专属 composables，避免一个文件继续膨胀。

## Target Structure

```text
src/
├── app/
│   ├── App.vue
│   ├── main.ts
│   └── styles.css
│
├── windows/
│   ├── settings/
│   │   ├── index.vue                  # 只负责设置窗口布局、当前菜单、面板切换
│   │   │
│   │   ├── model/
│   │   │   └── navigation.ts           # SettingsSection、SettingsNavItem、sections、panel width
│   │   │
│   │   ├── composables/
│   │   │   ├── useSettingsWindow.ts    # 标题栏拖动、最小化、最大化、隐藏、焦点同步
│   │   │   ├── useSettingsStartup.ts   # 初始化 Store、监听托盘、批量恢复 Box 窗口
│   │   │   ├── useSettingsBoxes.ts     # 新建、打开、锁定、删除 Box
│   │   │   └── useSettingsActions.ts   # 自启、收纳目录、迁移、通用错误收口
│   │   │
│   │   ├── components/
│   │   │   ├── SettingsHeader.vue
│   │   │   ├── SettingsSidebar.vue
│   │   │   ├── SettingSection.vue      # 通用页面外壳：标题、说明、宽度
│   │   │   ├── SettingGroup.vue        # 通用分组容器
│   │   │   ├── SettingRow.vue          # 通用设置行
│   │   │   ├── SettingSwitch.vue       # 通用开关
│   │   │   └── SettingSlider.vue       # 通用滑块 + 数值 + 重置
│   │   │
│   │   └── panels/
│   │       ├── BoxesPanel.vue          # Box 列表、新建、打开、锁定、删除、刷新、收纳根目录
│   │       ├── FilePanel.vue           # 规则显示：拖入/拖出/同名/删除/标签/打开
│   │       ├── WindowPanel.vue         # 窗口启动：边缘吸附、吸附距离、按网格调整大小、开机自启
│   │       ├── AppearancePanel.vue     # 设置页主题、Box 主题、透明度、动画、圆角、图标密度
│   │       └── AboutPanel.vue          # 仅作为面板保留；是否放进菜单由 navigation.ts 控制
│   │
│   ├── desktop/
│   ├── boxContextMenu/
│   └── dragPreview/
│
├── entities/
│   ├── appSettings/
│   │   ├── api.ts
│   │   ├── defaults.ts
│   │   ├── groups.ts                  # 设置分组常量：主题选项、策略选项、滑块配置
│   │   └── types.ts
│   ├── desktopBox/
│   └── desktopItem/
│
└── shared/
    ├── ipc/
    ├── storage/
    └── ui/
        └── SegmentedControl.vue
```

## Settings Menus

设置侧栏建议显示 5 个主菜单：

```text
1. Box
2. 规则显示
3. 窗口启动
4. 外观
5. 关于
```

侧栏实际展示使用四字以内短文案；`Box` 同时承载 Box 列表和收纳根目录，`规则显示` 合并文件策略和标签配置，`窗口启动` 合并窗口行为和自启配置。

## Key Changes

- `windows/settings/index.vue` 从 500+ 行收缩为壳组件：初始化 composables、渲染 sidebar/header、根据 `activeSection` 切换 panels。
- 原 `BoxDisplayPanel.vue` 拆为 3 个面板：`BoxesPanel`、`FilePanel`、`WindowPanel`；其中收纳根目录操作合并进 `BoxesPanel`，不再保留独立收纳菜单。
- 原 `AppearancePanel.vue` 保留，但只管真正的视觉项；不要再混入文件名规则或窗口行为。
- 把重复的设置卡片、行、开关、滑块抽成 settings 内部组件，不放进 `shared/ui`，因为它们带有设置页视觉和文案密度，不是全应用通用组件。
- 设置弹窗整体禁用文本框选，避免拖拽窗口或点击控件时误选中文案。
- `entities/appSettings/groups.ts` 只放跨多个 settings 面板复用的选项和分组配置；单面板独占的小常量仍留在对应 panel 内。

## Test Plan

- 运行 `pnpm exec vue-tsc --noEmit`。
- 手测设置页 5 个菜单切换（Box、规则显示、窗口启动、外观、关于）、托盘唤起、新建 Box、删除 Box、开机自启、选择/迁移收纳目录、主题切换、滑块重置、设置页文本不可框选。
- 确认前端 `invoke` 命令名不变，不影响刚拆完的 Rust 后端。

## Assumptions

- 本次只调整前端目录和设置弹窗结构，不改设置字段、不做数据库迁移。
- 旧组件拆空后直接删除，不保留 re-export 兼容入口。
- 所有新增/迁移的 Vue、TypeScript 文件保持 UTF-8，并按项目要求补充 JSDoc/业务意图注释。
