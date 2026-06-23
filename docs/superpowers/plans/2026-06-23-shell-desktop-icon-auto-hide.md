# 系统桌面图标自动隐藏实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 实现“系统桌面图标进入 Box 后自动隐藏原生图标，离开所有 Box 或退出 Dasktop 后按原状态恢复；下次启动后重新接管”的收纳能力。

**架构：** 后端新增 Windows Shell 桌面图标显示状态读写模块，前端在 Box 虚拟项引用变化后统一同步。SQLite 记录 Dasktop 自动接管前的可见状态，保证移出 Box、关闭总开关和正常退出时只恢复 Dasktop 自己隐藏过的图标。

**技术栈：** Tauri 2、Rust `windows` crate、Vue 3、Pinia、SQLite via `tauri-plugin-sql`。

---

## 文件结构

- 修改：`src-tauri/Cargo.toml`，启用 `Win32_System_Registry`。
- 创建：`src-tauri/src/infrastructure/windows/shell_desktop_icon_visibility.rs`，封装 Windows 注册表读写和 Explorer 刷新。
- 修改：`src-tauri/src/infrastructure/windows/mod.rs`，导出新 Windows 能力模块。
- 修改：`src-tauri/src/services/desktop_item.rs`，提供服务层函数。
- 修改：`src-tauri/src/services/app_lifecycle.rs`，应用退出前恢复接管过的系统桌面图标。
- 修改：`src-tauri/src/commands/desktop_item.rs`，新增 Tauri 命令。
- 修改：`src-tauri/src/lib.rs`，注册新命令。
- 修改：`src/entities/appSettings/types.ts`、`src/entities/appSettings/defaults.ts`、`src/shared/storage/database.ts`，新增总开关设置。
- 修改：`src/shared/storage/database.ts`，新增自动隐藏记录表和读取所有 Box 虚拟项引用的函数。
- 修改：`src/entities/desktopItem/api.ts`，新增前端同步 API。
- 创建：`src/entities/desktopItem/shellDesktopIconVisibilitySync.ts`，集中处理引用计数、自动隐藏记录和后端同步。
- 修改：`src/windows/desktop/composables/useBoxFileItems.ts`，在追加/移除 Shell 虚拟项后触发同步。
- 修改：`src/entities/desktopBox/store.ts`，初始化和设置切换时触发同步。
- 修改：`src/entities/appSettings/groups.ts`，增加总开关入口。
- 修改：`CHANGELOG.md`，补充功能说明。

## 任务 1：后端 Windows 图标显示能力

**文件：**
- 修改：`src-tauri/Cargo.toml`
- 创建：`src-tauri/src/infrastructure/windows/shell_desktop_icon_visibility.rs`
- 修改：`src-tauri/src/infrastructure/windows/mod.rs`
- 修改：`src-tauri/src/services/desktop_item.rs`
- 修改：`src-tauri/src/services/app_lifecycle.rs`
- 修改：`src-tauri/src/commands/desktop_item.rs`
- 修改：`src-tauri/src/lib.rs`

- [x] **步骤 1：启用 Registry 特性**

在 `src-tauri/Cargo.toml` 的 `windows.features` 中加入：

```toml
"Win32_System_Registry",
```

- [x] **步骤 2：创建 Windows 注册表封装**

创建 `shell_desktop_icon_visibility.rs`，实现：

```rust
pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String>;
pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String>;
```

实现要点：
- `shell_id` 映射到 CLSID。
- 读取 `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\HideDesktopIcons\NewStartPanel`。
- 写入 `NewStartPanel` 和 `ClassicStartMenu` 两个路径。
- DWORD `1` 表示隐藏，`0` 表示显示。
- 值缺失时使用默认可见性：`recycle-bin = true`，其他当前支持项为 `false`。
- 写入后调用 `SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None)` 刷新 Explorer。

- [x] **步骤 3：接入命令层**

新增两个 Tauri 命令：

```rust
#[tauri::command]
pub fn get_shell_desktop_icon_visible(shell_id: String) -> Result<bool, String>;

#[tauri::command]
pub fn set_shell_desktop_icon_visible(shell_id: String, visible: bool) -> Result<(), String>;
```

在 `lib.rs` 的 `generate_handler!` 注册。

- [x] **步骤 4：验证后端编译**

运行：

```bash
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
```

预期：编译通过。

- [x] **步骤 5：接入退出恢复**

在 `RunEvent::ExitRequested` 中读取自动隐藏记录，按 `previous_visible` 恢复 Windows 原生系统桌面图标，并清理已成功恢复的记录。恢复失败的记录保留，避免下一次启动丢失接管前状态。

## 任务 2：前端设置和持久化记录

**文件：**
- 修改：`src/entities/appSettings/types.ts`
- 修改：`src/entities/appSettings/defaults.ts`
- 修改：`src/shared/storage/database.ts`

- [x] **步骤 1：新增设置字段**

在 `AppSettings` 新增：

```ts
autoHideNativeShellIcons: boolean;
```

在 `APP_SETTING_KEYS`、`DEFAULT_APP_SETTINGS` 和布尔设置解析中补齐，默认值为 `true`。

- [x] **步骤 2：新增自动隐藏记录表**

在 `database.ts` 新增表：

```sql
CREATE TABLE IF NOT EXISTS shell_icon_visibility_records (
  shell_id TEXT PRIMARY KEY,
  previous_visible INTEGER NOT NULL,
  managed_hidden INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
)
```

- [x] **步骤 3：新增数据库函数**

实现：

```ts
export interface ShellIconVisibilityRecord {
  managedHidden: boolean;
  previousVisible: boolean;
  shellId: string;
}

export async function loadShellIconVisibilityRecords(): Promise<ShellIconVisibilityRecord[]>;
export async function saveShellIconVisibilityRecord(record: ShellIconVisibilityRecord): Promise<void>;
export async function deleteShellIconVisibilityRecord(shellId: string): Promise<void>;
export async function loadAllBoxVirtualItemIds(): Promise<string[]>;
```

- [x] **步骤 4：验证类型**

运行：

```bash
pnpm build
```

预期：`vue-tsc --noEmit` 通过。

## 任务 3：同步组合式逻辑和 Box 引用变化接入

**文件：**
- 修改：`src/entities/desktopItem/api.ts`
- 创建：`src/entities/desktopItem/shellDesktopIconVisibilitySync.ts`
- 修改：`src/windows/desktop/composables/useBoxFileItems.ts`
- 修改：`src/windows/desktop/index.vue`
- 修改：`src/entities/desktopBox/store.ts`

- [x] **步骤 1：新增前端 API**

在 `desktopItem/api.ts` 新增：

```ts
export function getShellDesktopIconVisible(shellId: string): Promise<boolean>;
export function setShellDesktopIconVisible(shellId: string, visible: boolean): Promise<void>;
```

- [x] **步骤 2：创建同步逻辑**

`useShellDesktopIconVisibilitySync.ts` 暴露：

```ts
export async function syncShellDesktopIconVisibility(options?: {
  autoHideEnabled?: boolean;
  forceRestoreManagedIcons?: boolean;
}): Promise<void>;
```

逻辑：
- 读取所有 Box 的 `shell_id` 引用并去重。
- 开关关闭或 `forceRestoreManagedIcons = true` 时，按 `previousVisible` 恢复 Dasktop 管理过的图标，并清理记录。
- 开关开启时，对仍被引用的图标隐藏；无引用且有管理记录的图标按 `previousVisible` 恢复。

- [x] **步骤 3：接入 Box 虚拟项变化**

在 `appendBoxShellItems` 和 `removeBoxShellItems` 后调用同步函数。传入当前设置中的 `autoHideNativeShellIcons`。

- [x] **步骤 4：接入启动同步**

在 Store 初始化完成、设置加载完成后执行一次同步，保证 Dasktop 重启后状态仍一致。

- [x] **步骤 5：验证构建**

运行：

```bash
pnpm build
```

预期：构建通过。

## 任务 4：设置页入口和发布记录

**文件：**
- 修改：`src/entities/appSettings/groups.ts`
- 修改：`CHANGELOG.md`

- [x] **步骤 1：添加设置开关**

在文件/整理相关设置中加入开关：

```text
收纳系统桌面图标后隐藏原生图标
```

说明：

```text
此电脑、回收站、网络等系统图标进入 Box 后，将从 Windows 桌面隐藏；从所有 Box 移除后再恢复。
```

- [x] **步骤 2：开关关闭时恢复托管图标**

设置变为 `false` 后调用同步函数的恢复路径；设置变为 `true` 后重新按当前 Box 引用隐藏。

- [x] **步骤 3：更新 CHANGELOG**

在 `0.2.2` 新增或优化段落补充系统桌面图标自动隐藏能力。

- [x] **步骤 4：最终验证**

运行：

```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
```

预期：前端构建和 Rust 检查均通过。
