# Box 图标与桌面隐藏 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:executing-plans 或逐任务执行此计划。

**目标：** 桌面图标拖入 Box 后，Box 内显示与桌面一致的原生图标，桌面列表同步消失。

**架构：** Rust 侧的桌面扫描结果新增 `iconDataUrl` 字段，用来承载 Windows 原生图标的 PNG data URL。前端 `DesktopItem` 类型、桌面图标组件和 Box 内图标网格都改为优先渲染该字段，未能提取时再回退到现有 Lucide 语义图标。Box 是否显示某个项目继续由 `itemPaths` 映射控制，不修改真实文件位置。

**技术栈：** Tauri 2、Rust、Windows API、Vue 3、Pinia、TypeScript、Tailwind CSS。

---

### 任务 1：扩展桌面扫描数据

**文件：**
- 修改：`src-tauri/src/desktop/types.rs`
- 修改：`src-tauri/src/desktop/scanner.rs`
- 修改：`src-tauri/src/desktop/mod.rs`

- [x] **步骤 1：为 `DesktopItem` 增加 `iconDataUrl` 字段**

```rust
pub struct DesktopItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: Option<String>,
    pub kind: DesktopItemKind,
    pub icon_data_url: Option<String>,
}
```

- [x] **步骤 2：在 `scan_desktop()` 中填充原生图标 data URL**

```rust
items.push(DesktopItem {
    id: stable_item_id(&path),
    name: entry.file_name().to_string_lossy().to_string(),
    path: path.to_string_lossy().to_string(),
    extension: path.extension().map(|value| value.to_string_lossy().to_string()),
    kind: resolve_item_kind(&path, metadata.is_dir()),
    icon_data_url: resolve_item_icon_data_url(&path),
});
```

- [x] **步骤 3：把图标解析封装在扫描模块内部**

```rust
#[cfg(target_os = "windows")]
fn resolve_item_icon_data_url(path: &Path) -> Option<String> {
    windows_icon::resolve_shell_icon_data_url(path).ok().flatten()
}
```

- [x] **步骤 4：验证 Rust 侧能继续编译**

运行：`cargo check --manifest-path src-tauri/Cargo.toml`
预期：通过，且 `DesktopSnapshot` JSON 仍然可序列化。

### 任务 2：前端复用原生图标并保持映射隐藏

**文件：**
- 修改：`src/shared/types/desktop.ts`
- 修改：`src/features/desktop/components/DesktopIcon.vue`
- 修改：`src/features/desktop/index.vue`
- 修改：`src/features/desktop/store/desktopStore.ts`

- [x] **步骤 1：为前端桌面类型补充 `iconDataUrl`**

```ts
export interface DesktopItem {
  id: string;
  name: string;
  path: string;
  extension: string | null;
  kind: DesktopItemKind;
  iconDataUrl: string | null;
}
```

- [x] **步骤 2：让桌面与 Box 统一使用 data URL 图标**

```vue
<img v-if="item.iconDataUrl" :src="item.iconDataUrl" :alt="item.name" class="size-9 rounded-[10px]" />
<span v-else>
  <Folder v-if="item.kind === 'folder'" :size="20" />
</span>
```

- [x] **步骤 3：保持 Box 内 itemPaths 渲染和桌面隐藏逻辑不变**

```ts
const unassignedItems = computed(() => {
  const assignedPaths = new Set(boxes.value.flatMap((box) => box.itemPaths));
  return items.value.filter((item) => !assignedPaths.has(item.path));
});
```

- [x] **步骤 4：验证前端构建通过**

运行：`pnpm build`
预期：TypeScript 类型与 Vue 模板都能通过。

### 任务 3：手动验证拖入效果

**文件：**
- 不新增代码，仅运行应用验证

- [ ] **步骤 1：启动应用并拖入桌面图标到 Box**

运行：`pnpm tauri dev`

- [ ] **步骤 2：确认 Box 内显示图标，桌面上该图标消失**

预期：同一项目在 Box 中可见，在桌面主列表中不再显示。

- [ ] **步骤 3：确认显示的是系统原生图标而不是通用占位图标**

预期：文件夹、快捷方式和常见程序文件的图标与 Windows 桌面保持一致。
