# Shell 虚拟桌面项实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 支持把 Windows 原生桌面图标（此电脑、回收站等 Shell 虚拟项）收纳进 Box，并保持普通文件拖放不退化。

**架构：** Rust 侧把桌面扫描扩展为“文件系统项 + Shell 虚拟项”，为 Shell 项生成稳定 `shell::<knownFolder>` 路径键和原生图标。前端继续用 `DesktopItem.path` 做映射主键，新增 `source` 字段区分打开方式；Box 映射和排序逻辑复用现有关联表。

**技术栈：** Tauri 2、Windows Shell API、Vue 3、Pinia、SQLite、TypeScript。

---

## 文件结构

- 修改 `src-tauri/src/desktop/types.rs`：给 `DesktopItem` 增加 `source` 与 `shell_id`，定义文件系统项和 Shell 虚拟项的序列化模型。
- 修改 `src-tauri/src/desktop/scanner.rs`：扫描桌面文件后追加“此电脑、回收站”等已知 Shell 项，并支持按 `shell::` 键解析项目。
- 修改 `src-tauri/src/commands/mod.rs`：`open_desktop_item` 和右键菜单根据 `shell_id` 走 Shell PIDL，不再要求所有项目必须 `Path::exists()`。
- 修改 `src/entities/desktopItem/types.ts`：同步新增字段，保持前端类型和 Rust camelCase 输出一致。
- 修改 `src/entities/desktopBox/store.ts`：让 `ensureItemsAvailable()` 能通过 `shell::` 键补齐 Shell 项，映射去重继续使用路径键。
- 修改 `src/windows/settings/index.vue`：恢复用户确认的可见创建，避免 Windows WebView2 隐藏创建时拖放目标注册异常。

## 任务 1：建模 Shell 虚拟项

**文件：**
- 修改：`src-tauri/src/desktop/types.rs`
- 修改：`src/entities/desktopItem/types.ts`

- [ ] **步骤 1：扩展 Rust 类型**

在 `src-tauri/src/desktop/types.rs` 中新增：

```rust
/// 桌面项来源决定打开和右键菜单走文件路径还是 Shell PIDL
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopItemSource {
    FileSystem,
    Shell,
}
```

并给 `DesktopItem` 增加字段：

```rust
pub source: DesktopItemSource,
pub shell_id: Option<String>,
```

- [ ] **步骤 2：同步前端类型**

在 `src/entities/desktopItem/types.ts` 中新增：

```ts
export type DesktopItemSource = "fileSystem" | "shell";
```

并给 `DesktopItem` 增加：

```ts
source: DesktopItemSource;
shellId: string | null;
```

- [ ] **步骤 3：运行类型检查**

运行：`pnpm build`

预期：此时 Rust 输出字段尚未填充，前端可能只在构建时通过；若 TypeScript 报字段缺失，继续任务 2 填充数据源。

## 任务 2：扫描并解析已知 Shell 图标

**文件：**
- 修改：`src-tauri/src/desktop/scanner.rs`

- [ ] **步骤 1：定义已知 Shell 项**

在 `scanner.rs` 增加常量：

```rust
struct KnownShellDesktopItem {
    id: &'static str,
    name: &'static str,
    parsing_name: &'static str,
}

const KNOWN_SHELL_DESKTOP_ITEMS: &[KnownShellDesktopItem] = &[
    KnownShellDesktopItem {
        id: "this-pc",
        name: "此电脑",
        parsing_name: "::{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
    },
    KnownShellDesktopItem {
        id: "recycle-bin",
        name: "回收站",
        parsing_name: "::{645FF040-5081-101B-9F08-00AA002F954E}",
    },
];
```

- [ ] **步骤 2：让桌面快照追加 Shell 项**

在 `scan_desktop()` 读取真实桌面文件后追加：

```rust
items.extend(KNOWN_SHELL_DESKTOP_ITEMS.iter().map(create_shell_desktop_item));
```

- [ ] **步骤 3：让 `scan_paths()` 支持 `shell::` 键**

在 `scan_paths()` 循环中先判断：

```rust
if let Some(shell_item) = create_shell_desktop_item_from_key(raw_path) {
    items.push(shell_item);
    continue;
}
```

- [ ] **步骤 4：构造 Shell 桌面项**

新增函数：

```rust
fn create_shell_desktop_item(item: &KnownShellDesktopItem) -> DesktopItem {
    DesktopItem {
        id: format!("shell_{}", item.id),
        name: item.name.to_string(),
        path: format!("shell::{}", item.id),
        extension: None,
        kind: DesktopItemKind::Shell,
        icon_data_url: resolve_item_icon_data_url(Path::new(item.parsing_name)),
        source: DesktopItemSource::Shell,
        shell_id: Some(item.id.to_string()),
    }
}
```

`DesktopItemKind` 同步增加 `Shell`。

## 任务 3：打开 Shell 项

**文件：**
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/src/desktop/scanner.rs`
- 修改：`src-tauri/src/desktop/mod.rs`

- [ ] **步骤 1：导出 Shell 解析函数**

在 `scanner.rs` 新增：

```rust
pub fn resolve_shell_parsing_name(shell_id: &str) -> Option<&'static str> {
    KNOWN_SHELL_DESKTOP_ITEMS
        .iter()
        .find(|item| item.id == shell_id)
        .map(|item| item.parsing_name)
}
```

并在 `desktop/mod.rs` 导出。

- [ ] **步骤 2：更新打开逻辑**

在 `commands::open_desktop_item` 中：

```rust
if let Some(shell_id) = path.strip_prefix("shell::") {
    let parsing_name = crate::desktop::resolve_shell_parsing_name(shell_id)
        .ok_or_else(|| "系统桌面项目类型暂不支持".to_string())?;
    return open_shell_item_with_system_default(parsing_name);
}
```

- [ ] **步骤 3：实现 ShellExecute 打开 Shell 项**

新增 Windows 函数：

```rust
#[cfg(target_os = "windows")]
fn open_shell_item_with_system_default(parsing_name: &str) -> Result<(), String> {
    let path_wide = parsing_name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(None, PCWSTR(operation_wide.as_ptr()), PCWSTR(path_wide.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL)
    };

    if result.0 as isize <= 32 {
        return Err(format!("系统无法打开该桌面项目，错误码 {}", result.0 as isize));
    }

    Ok(())
}
```

## 任务 4：右键菜单兼容 Shell 项

**文件：**
- 修改：`src-tauri/src/commands/mod.rs`

- [ ] **步骤 1：跳过 `Path::exists()` 硬校验**

`show_native_item_context_menu()` 中先判断 `shell::`：

```rust
if let Some(shell_id) = path.strip_prefix("shell::") {
    let parsing_name = crate::desktop::resolve_shell_parsing_name(&shell_id)
        .ok_or_else(|| "系统桌面项目类型暂不支持".to_string())?;
    return show_native_context_menu_for_parsing_name(&window, parsing_name, screen_x, screen_y);
}
```

- [ ] **步骤 2：抽出 parsing name 右键入口**

新增：

```rust
#[cfg(target_os = "windows")]
fn show_native_context_menu_for_parsing_name(
    window: &tauri::WebviewWindow,
    parsing_name: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|error| format!("无法获取窗口句柄：{error}"))?;
    let wide_path = parsing_name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SHParseDisplayName(PCWSTR(wide_path.as_ptr()), None, &mut pidl, 0, None)
            .map_err(|error| format!("系统无法解析该桌面项目：{error}"))?;
        let result = show_context_menu_from_pidl(hwnd, pidl, screen_x, screen_y);
        CoTaskMemFree(Some(pidl as *const _));
        result
    }
}
```

## 任务 5：恢复启动可见创建

**文件：**
- 修改：`src/windows/settings/index.vue`

- [ ] **步骤 1：恢复 `visible: true`**

在启动批量打开 Box 的 `openBoxWindow()` 参数中设置：

```ts
/**
 * Windows WebView2 的文件拖放目标在创建阶段绑定到子 HWND；隐藏创建时可能导致 DropTarget 注册失败。
 * 启动恢复保留快照复用，但让 Box 可见创建以保证普通文件和 Shell 项拖入都能命中。
 */
visible: true,
```

## 任务 6：验证

**文件：**
- 修改：无

- [ ] **步骤 1：Rust 格式化**

运行：`cargo fmt --manifest-path src-tauri/Cargo.toml`

预期：成功。

- [ ] **步骤 2：Rust 检查**

运行：`cargo check --manifest-path src-tauri/Cargo.toml`

预期：成功。

- [ ] **步骤 3：前端构建**

运行：`pnpm build`

预期：成功。

- [ ] **步骤 4：手动验证**

启动当前应用后验证：

1. 拖普通文件到 Box，鼠标不是禁用状态，文件出现在 Box。
2. 拖 `此电脑` 到 Box，鼠标不是禁用状态，Box 中显示 `此电脑`。
3. 双击 Box 内 `此电脑`，系统打开此电脑。
4. 拖 `回收站` 到 Box，Box 中显示 `回收站`。
5. 双击 Box 内 `回收站`，系统打开回收站。
