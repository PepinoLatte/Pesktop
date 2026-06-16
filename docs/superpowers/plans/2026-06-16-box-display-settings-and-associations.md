# Box 显示设置与关联表 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:executing-plans 或逐任务执行此计划。

**目标：** 将 Box 映射改为独立关联表，Box 图标按 Windows 默认展示逻辑渲染，并在设置页新增全局 Box 显示配置。

**架构：** 数据层把 `boxes.item_paths` 拆成 `box_items` 关联表，便于单项查询、删除和去重。桌面扫描层继续提供真实文件元数据，但前端不再只渲染统一图标，而是按 Shell 默认行为展示缩略图、文件夹、快捷方式和普通文件。设置层新增一个专门的 Box 菜单，快捷方式箭头和后缀显示模式都作为全局共享设置持久化。

**技术栈：** Tauri 2、Rust、Windows API、Vue 3、Pinia、TypeScript、Tailwind CSS。

---

### 任务 1：把 Box 映射拆成关联表

**文件：**
- 修改：`src/shared/storage/database.ts`
- 修改：`src/shared/types/desktop.ts`
- 修改：`src/features/desktop/store/desktopStore.ts`
- 修改：`src-tauri/src/desktop/types.rs`

- [x] **步骤 1：把 `DesktopBox` 改成不再携带 `itemPaths`**

```ts
export interface DesktopBox {
  id: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
}
```

- [x] **步骤 2：新增 `box_items` 表并把加载/保存逻辑改成关联表**

```ts
await database.execute(`
  CREATE TABLE IF NOT EXISTS box_items (
    box_id TEXT NOT NULL,
    item_path TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (box_id, item_path)
  )
`);
```

- [x] **步骤 3：把删除、查询、批量映射改成按关联表操作**

```ts
await database.execute("DELETE FROM box_items WHERE box_id = $1", [boxId]);
await database.execute("DELETE FROM box_items WHERE item_path IN ($1, $2)", itemPaths);
```

- [x] **步骤 4：验证 SQLite 层和 Store 类型一致**

运行：`cargo check --manifest-path src-tauri/Cargo.toml`
运行：`pnpm build`

### 任务 2：让 Box 按 Windows 默认逻辑显示内容

**文件：**
- 修改：`src-tauri/src/desktop/scanner.rs`
- 修改：`src-tauri/src/desktop/types.rs`
- 修改：`src/features/desktop/components/DesktopIcon.vue`
- 修改：`src/features/desktop/index.vue`

- [x] **步骤 1：为扫描结果补充可用于缩略图/默认图标的展示字段**

```rust
pub struct DesktopItem {
    pub icon_data_url: Option<String>,
    pub display_kind: DesktopDisplayKind,
}
```

- [x] **步骤 2：前端组件按缩略图优先、图标兜底的顺序渲染**

```vue
<img v-if="item.previewUrl" :src="item.previewUrl" :alt="item.name" />
<img v-else-if="item.iconDataUrl" :src="item.iconDataUrl" :alt="item.name" />
```

- [x] **步骤 3：去掉 Box 内图标外框，并让点击直接打开项目**

```ts
function openDesktopItem(item: DesktopItem): Promise<void> {
  return openDesktopPath(item.path);
}
```

- [x] **步骤 4：验证 Box 内视觉和点击行为**

运行：`cargo check --manifest-path src-tauri/Cargo.toml`
运行：`pnpm build`

### 任务 3：新增全局 Box 显示菜单和分段控件

**文件：**
- 修改：`src/shared/types/desktop.ts`
- 修改：`src/shared/storage/database.ts`
- 修改：`src/features/settings/types.ts`
- 创建：`src/features/settings/components/SegmentedControl.vue`
- 创建：`src/features/settings/components/BoxDisplayPanel.vue`
- 修改：`src/features/settings/components/AppearancePanel.vue`
- 修改：`src/features/settings/index.vue`

- [x] **步骤 1：新增 Box 显示设置字段**

```ts
export interface AppSettings {
  showShortcutArrow: boolean;
  nameDisplayMode: "full" | "no-shortcut-ext" | "hidden";
}
```

- [x] **步骤 2：抽出可复用的分段控件**

```vue
<button
  v-for="option in options"
  :key="option.value"
  :class="option.value === modelValue ? activeClass : inactiveClass"
/>
```

- [x] **步骤 3：设置页新增 Box 菜单并接入新设置项**

```vue
<BoxDisplayPanel
  :settings="desktopStore.settings"
  @show-shortcut-arrow-change="desktopStore.updateShowShortcutArrow"
  @name-display-mode-change="desktopStore.updateNameDisplayMode"
/>
```

- [x] **步骤 4：验证设置页交互和持久化**

运行：`pnpm build`

### 任务 4：补齐打开文件与最终验证

**文件：**
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/src/lib.rs`
- 修改：`src/features/desktop/components/DesktopIcon.vue`

- [x] **步骤 1：新增 Rust 打开路径命令**

```rust
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    // Shell 打开逻辑
}
```

- [x] **步骤 2：前端点击图标调用打开命令**

```ts
await invoke("open_desktop_item", { path: item.path });
```

- [x] **步骤 3：运行最终回归验证**

运行：`cargo check --manifest-path src-tauri/Cargo.toml`
运行：`pnpm build`
预期：构建通过，Box 显示、打开和设置项都能正常工作。
