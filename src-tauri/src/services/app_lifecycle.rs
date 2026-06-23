//! 应用生命周期服务负责把命令层的请求转交给 Tauri 外设，并隐藏托盘同步细节。

use crate::infrastructure::tauri::tray;
use crate::infrastructure::windows::shell_desktop_icon_visibility;
use sqlx::{Executor, Row, SqlitePool};
use tauri::{AppHandle, Manager};
use tauri_plugin_sql::{DbInstances, DbPool};

const APP_DATABASE_URL: &str = "sqlite:dasktop.db";
const SHELL_ICON_VISIBILITY_RECORDS_TABLE: &str = "shell_icon_visibility_records";

/// Dasktop 接管过的系统桌面图标状态，退出恢复时按首次接管前的状态还原 Windows 桌面。
struct ShellIconVisibilityRecord {
    managed_hidden: bool,
    previous_visible: bool,
    shell_id: String,
}

/// 读取插件中的系统自启状态，供设置页命令使用。
pub fn resolve_autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    tray::resolve_autostart_enabled(app)
}

/// 写入系统自启状态并同步所有 UI 入口，避免设置页和托盘出现互相矛盾的勾选状态。
pub fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    tray::set_autostart_enabled(app, enabled)
}

/// 应用正常退出前释放 Dasktop 对系统桌面图标的接管，让 Windows 桌面回到接管前状态。
pub fn restore_shell_desktop_icons_before_exit(app: &AppHandle) -> Result<(), String> {
    tauri::async_runtime::block_on(restore_shell_desktop_icons_before_exit_async(app))
}

async fn restore_shell_desktop_icons_before_exit_async(app: &AppHandle) -> Result<(), String> {
    let Some(pool) = resolve_app_sqlite_pool(app).await? else {
        return Ok(());
    };
    ensure_shell_icon_visibility_record_storage(&pool).await?;
    let records = load_shell_icon_visibility_records(&pool).await?;
    let mut errors = Vec::new();

    for record in records {
        if let Err(error) = restore_shell_icon_visibility_record(&pool, &record).await {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

async fn resolve_app_sqlite_pool(app: &AppHandle) -> Result<Option<SqlitePool>, String> {
    let db_instances = app.state::<DbInstances>();
    let instances = db_instances.0.read().await;
    let Some(db_pool) = instances.get(APP_DATABASE_URL) else {
        return Ok(None);
    };

    let DbPool::Sqlite(pool) = db_pool;
    Ok(Some(pool.clone()))
}

async fn ensure_shell_icon_visibility_record_storage(pool: &SqlitePool) -> Result<(), String> {
    let query = format!(
        "
        CREATE TABLE IF NOT EXISTS {SHELL_ICON_VISIBILITY_RECORDS_TABLE} (
          shell_id TEXT PRIMARY KEY,
          previous_visible INTEGER NOT NULL,
          managed_hidden INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        )
        "
    );
    pool.execute(sqlx::query(&query))
        .await
        .map_err(|error| format!("初始化系统桌面图标接管记录失败：{error}"))?;

    Ok(())
}

async fn load_shell_icon_visibility_records(
    pool: &SqlitePool,
) -> Result<Vec<ShellIconVisibilityRecord>, String> {
    let query = format!(
        "
        SELECT shell_id, previous_visible, managed_hidden
        FROM {SHELL_ICON_VISIBILITY_RECORDS_TABLE}
        ORDER BY updated_at ASC
        "
    );
    let rows = sqlx::query(&query)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("读取系统桌面图标接管记录失败：{error}"))?;

    rows.into_iter()
        .map(|row| {
            Ok(ShellIconVisibilityRecord {
                managed_hidden: read_sqlite_bool(&row, "managed_hidden")?,
                previous_visible: read_sqlite_bool(&row, "previous_visible")?,
                shell_id: row
                    .try_get::<String, _>("shell_id")
                    .map_err(|error| format!("读取系统桌面图标 ID 失败：{error}"))?,
            })
        })
        .collect()
}

async fn restore_shell_icon_visibility_record(
    pool: &SqlitePool,
    record: &ShellIconVisibilityRecord,
) -> Result<(), String> {
    if record.managed_hidden {
        shell_desktop_icon_visibility::set_shell_desktop_icon_visible(
            &record.shell_id,
            record.previous_visible,
        )?;
    }

    delete_shell_icon_visibility_record(pool, &record.shell_id).await
}

async fn delete_shell_icon_visibility_record(
    pool: &SqlitePool,
    shell_id: &str,
) -> Result<(), String> {
    let query = format!("DELETE FROM {SHELL_ICON_VISIBILITY_RECORDS_TABLE} WHERE shell_id = ?");
    sqlx::query(&query)
        .bind(shell_id)
        .execute(pool)
        .await
        .map_err(|error| format!("清理系统桌面图标接管记录失败：{error}"))?;

    Ok(())
}

fn read_sqlite_bool(row: &sqlx::sqlite::SqliteRow, column_name: &str) -> Result<bool, String> {
    row.try_get::<i64, _>(column_name)
        .map(|value| value == 1)
        .map_err(|error| format!("读取系统桌面图标布尔状态失败：{error}"))
}
