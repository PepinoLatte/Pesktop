# 后端目录结构拆分方案

## Summary

建议采用偏 Spring Boot 三层的 Rust/Tauri 分层：`commands` 作为 Controller，`services` 承载业务用例，`domain` 放领域类型和策略，`infrastructure` 放文件系统、Windows Shell、Tauri 托盘等系统适配。前端 `invoke` 的命令名保持不变，不需要改 TS API。

## Target Structure

```text
src-tauri/src/
├── main.rs
├── lib.rs
│
├── commands/                 # Controller：只做 Tauri 参数接收、窗口句柄提取、错误透传
│   ├── mod.rs
│   ├── app.rs                # 自启、鼠标状态等应用级命令
│   ├── box_folder.rs         # Box 文件夹生命周期、拖入拖出命令
│   ├── desktop_item.rs       # 文件项扫描、打开、右键、重命名、删除命令
│   └── native_drop.rs        # Box 原生 DropTarget 注册命令
│
├── services/                 # Service：业务流程编排，类似 Spring Service
│   ├── mod.rs
│   ├── app_lifecycle.rs      # 开机自启状态、托盘同步
│   ├── box_folder.rs         # 创建/删除/迁移 Box 文件夹，拖入拖出策略编排
│   └── desktop_item.rs       # 桌面路径快照、Box 文件夹扫描、文件项操作
│
├── domain/                   # Domain：纯类型、策略、跨层契约
│   ├── mod.rs
│   ├── app_settings.rs       # 设置 key 常量
│   ├── box_policy.rs         # BoxDeletePolicy / BoxDropAction / BoxConflictPolicy
│   └── desktop_item.rs       # DesktopSnapshot / DesktopItem / DesktopItemKind
│
└── infrastructure/           # Infrastructure：系统 API、文件系统、Tauri 外设
    ├── mod.rs
    │
    ├── tauri/
    │   ├── mod.rs
    │   └── tray.rs           # 托盘菜单、托盘事件、窗口展示、优雅退出
    │
    ├── filesystem/
    │   ├── mod.rs
    │   ├── box_folder.rs     # 收纳根目录、Box 文件夹创建/删除/迁移
    │   ├── naming.rs         # 文件名/Box 目录名校验、同名重命名策略
    │   └── transfer.rs       # copy/move/map、冲突处理、失败回滚
    │
    └── windows/
        ├── mod.rs
        ├── desktop_path.rs   # Known Folder 桌面路径解析
        ├── folder_dialog.rs  # Windows 原生文件夹选择器
        ├── shell_context.rs  # Explorer 原生右键菜单
        ├── shell_file.rs     # ShellExecute、回收站、快捷方式创建
        ├── shell_icon.rs     # Shell 缩略图/图标提取
        ├── native_drop.rs    # OLE DropTarget
        └── mouse.rs          # GetAsyncKeyState 左键状态
```

## Key Mapping

- `commands/mod.rs` 拆成 4 个命令文件；其中 Windows 右键菜单逻辑下沉到 `infrastructure/windows/shell_context.rs`，命令层只保留薄封装。
- `desktop/types.rs` 搬到 `domain/desktop_item.rs`；`folder_ops.rs` 里的三个策略 enum 搬到 `domain/box_policy.rs`。
- `desktop/folder_ops.rs` 拆到 `services/box_folder.rs`、`infrastructure/filesystem/*` 和 `infrastructure/windows/shell_file.rs / folder_dialog.rs`。
- `desktop/scanner.rs` 拆到 `services/desktop_item.rs`、`infrastructure/windows/desktop_path.rs`、`infrastructure/windows/shell_icon.rs`。
- `desktop/native_drop.rs` 搬到 `infrastructure/windows/native_drop.rs`。
- `app_tray.rs` 搬到 `infrastructure/tauri/tray.rs`，自启读写由 `services/app_lifecycle.rs` 对外暴露。
- `app_settings.rs` 搬到 `domain/app_settings.rs`。
- 旧的 `desktop/`、`app_tray.rs`、`app_settings.rs` 在迁移完成后直接删除，不保留兼容转发入口。

## Public API

- 保持所有 `#[tauri::command]` 函数名不变：例如 `create_box_folder`、`list_box_folder_items`、`set_autostart_enabled`。
- `src-tauri/src/lib.rs` 仍统一注册插件、托盘和 `generate_handler!`，只是引用新的 `commands::*`。
- 前端 `src/entities/*/api.ts` 暂不需要改命令名，只在后续前端结构拆分时再调整自身目录。

## Test Plan

- 运行 `cargo fmt`。
- 运行 `cargo check`。
- 重点手测：打开设置页、托盘菜单、开机自启切换、选择收纳目录、新建/删除/迁移 Box、拖入/拖出文件、文件右键菜单、文件图标显示、Box 原生拖放注册与注销。

## Assumptions

- 这次只调整 Rust 后端结构，不先动前端设置弹窗。
- 项目未上线，所以按项目规则删除旧模块，不保留历史兼容层。
- 文件和注释继续保持 UTF-8，迁移 Rust public 类型/函数时保留或补齐 rustdoc。
