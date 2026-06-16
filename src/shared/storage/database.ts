import Database from "@tauri-apps/plugin-sql";
import type { AppSettings, DesktopBox, ThemeMode } from "../types/desktop";

/**
 * 默认设置只包含当前版本真实生效的字段，历史字段不会被继续写回数据库。
 */
const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  snapToEdges: true,
  snapThreshold: 20,
  showItemLabels: true,
};

let databasePromise: Promise<Database> | null = null;

/**
 * SQLite 连接复用可以避免拖拽过程中反复打开数据库。
 */
async function getDatabase(): Promise<Database> {
  if (!databasePromise) {
    databasePromise = Database.load("sqlite:dasktop.db");
  }

  return databasePromise;
}

/**
 * 初始化 V1 需要的最小表结构；分组只保存映射，不触碰真实桌面文件。
 */
export async function initializeStorage(): Promise<void> {
  const database = await getDatabase();

  await database.execute(`
    CREATE TABLE IF NOT EXISTS boxes (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      x INTEGER NOT NULL,
      y INTEGER NOT NULL,
      width INTEGER NOT NULL,
      height INTEGER NOT NULL,
      item_paths TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    )
  `);

  await database.execute(`
    CREATE TABLE IF NOT EXISTS app_settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    )
  `);
}

/**
 * 读取所有 Box 布局和映射；真实桌面文件仍由桌面快照提供。
 */
export async function loadBoxes(): Promise<DesktopBox[]> {
  const database = await getDatabase();
  const rows = await database.select<Array<Record<string, unknown>>>(`
    SELECT id, title, x, y, width, height, item_paths
    FROM boxes
    ORDER BY updated_at ASC
  `);

  return rows.map((row) => ({
    id: String(row.id),
    title: String(row.title),
    x: Number(row.x),
    y: Number(row.y),
    width: Number(row.width),
    height: Number(row.height),
    itemPaths: JSON.parse(String(row.item_paths)) as string[],
  }));
}

/**
 * 保存单个 Box 状态，拖动、缩放和映射变化都会通过这里落库。
 */
export async function saveBox(box: DesktopBox): Promise<void> {
  const database = await getDatabase();

  await database.execute(
    `
      INSERT INTO boxes (id, title, x, y, width, height, item_paths, updated_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        x = excluded.x,
        y = excluded.y,
        width = excluded.width,
        height = excluded.height,
        item_paths = excluded.item_paths,
        updated_at = excluded.updated_at
    `,
    [
      box.id,
      box.title,
      box.x,
      box.y,
      box.width,
      box.height,
      JSON.stringify(box.itemPaths),
      Date.now(),
    ],
  );
}

/**
 * 删除 Box 记录只影响 Dasktop 映射，不删除桌面文件本体。
 */
export async function deleteBoxRecord(boxId: string): Promise<void> {
  const database = await getDatabase();

  await database.execute("DELETE FROM boxes WHERE id = $1", [boxId]);
}

/**
 * 读取当前设置，并过滤掉旧版本残留或非法的设置值。
 */
export async function loadSettings(): Promise<AppSettings> {
  const database = await getDatabase();
  const rows = await database.select<Array<Record<string, unknown>>>(`
    SELECT key, value FROM app_settings
  `);
  const settings = { ...DEFAULT_SETTINGS };
  const supportedKeys = new Set<keyof AppSettings>([
    "theme",
    "snapToEdges",
    "snapThreshold",
    "showItemLabels",
  ]);

  for (const row of rows) {
    const key = String(row.key) as keyof AppSettings;
    if (supportedKeys.has(key)) {
      const value = JSON.parse(String(row.value));
      settings[key] = sanitizeSettingValue(key, value) as never;
    }
  }

  return settings;
}

/**
 * 设置读取只接受当前版本定义的值，旧主题和旧字段不会被迁移回新状态。
 */
function sanitizeSettingValue<Key extends keyof AppSettings>(
  key: Key,
  value: unknown,
): AppSettings[Key] {
  if (key === "theme") {
    return (isThemeMode(value) ? value : DEFAULT_SETTINGS.theme) as AppSettings[Key];
  }

  if (key === "snapToEdges" || key === "showItemLabels") {
    return (typeof value === "boolean" ? value : DEFAULT_SETTINGS[key]) as AppSettings[Key];
  }

  if (key === "snapThreshold") {
    return (typeof value === "number" ? value : DEFAULT_SETTINGS.snapThreshold) as AppSettings[Key];
  }

  return DEFAULT_SETTINGS[key];
}

/**
 * 主题枚举只允许当前三种值，避免历史状态继续污染 UI。
 */
function isThemeMode(value: unknown): value is ThemeMode {
  return value === "light" || value === "system" || value === "dark";
}

/**
 * 按键保存设置，保持设置项独立更新，避免整行 JSON 合并冲突。
 */
export async function saveSetting<Key extends keyof AppSettings>(
  key: Key,
  value: AppSettings[Key],
): Promise<void> {
  const database = await getDatabase();

  await database.execute(
    `
      INSERT INTO app_settings (key, value, updated_at)
      VALUES ($1, $2, $3)
      ON CONFLICT(key) DO UPDATE SET
        value = excluded.value,
        updated_at = excluded.updated_at
    `,
    [key, JSON.stringify(value), Date.now()],
  );
}
