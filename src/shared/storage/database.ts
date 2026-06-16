import Database from "@tauri-apps/plugin-sql";
import type {
  AppSettings,
  DesktopBox,
  DesktopBoxItem,
} from "../types/desktop";
import {
  APP_SETTINGS_STORAGE,
  DEFAULT_APP_SETTINGS,
  DESKTOP_NAME_DISPLAY_MODES,
  SUPPORTED_APP_SETTING_KEYS,
  THEME_MODES,
  APP_SETTING_KEYS,
} from "../config/appSettings";

let databasePromise: Promise<Database> | null = null;

/**
 * SQLite 连接复用可以避免拖拽过程中反复打开数据库。
 */
async function getDatabase(): Promise<Database> {
  if (!databasePromise) {
    databasePromise = Database.load(APP_SETTINGS_STORAGE.databaseUrl);
  }

  return databasePromise;
}

/**
 * 初始化 V1 需要的最小表结构；分组只保存映射，不触碰真实桌面文件。
 */
export async function initializeStorage(): Promise<void> {
  const database = await getDatabase();

  await resetLegacyBoxSchema(database);

  await database.execute(`
    CREATE TABLE IF NOT EXISTS ${APP_SETTINGS_STORAGE.tables.boxes} (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      x INTEGER NOT NULL,
      y INTEGER NOT NULL,
      width INTEGER NOT NULL,
      height INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
    )
  `);

  await database.execute(`
    CREATE TABLE IF NOT EXISTS ${APP_SETTINGS_STORAGE.tables.boxItems} (
      box_id TEXT NOT NULL,
      item_path TEXT NOT NULL,
      updated_at INTEGER NOT NULL,
      PRIMARY KEY (box_id, item_path),
      FOREIGN KEY (box_id) REFERENCES ${APP_SETTINGS_STORAGE.tables.boxes}(id) ON DELETE CASCADE
    )
  `);

  await database.execute(`
    CREATE TABLE IF NOT EXISTS ${APP_SETTINGS_STORAGE.tables.appSettings} (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    )
  `);
}

/**
 * 发现旧的 item_paths 列时直接重建 Box 表，避免新旧映射模型混用造成删除和查询语义分裂。
 */
async function resetLegacyBoxSchema(database: Database): Promise<void> {
  const columns = await database.select<Array<Record<string, unknown>>>(
    `PRAGMA table_info(${APP_SETTINGS_STORAGE.tables.boxes})`,
  );
  const hasLegacyItemPaths = columns.some((column) => String(column.name) === "item_paths");

  if (!hasLegacyItemPaths) {
    return;
  }

  await database.execute(`DROP TABLE IF EXISTS ${APP_SETTINGS_STORAGE.tables.boxItems}`);
  await database.execute(`DROP TABLE IF EXISTS ${APP_SETTINGS_STORAGE.tables.boxes}`);
}

/**
 * 读取所有 Box 布局和映射；真实桌面文件仍由桌面快照提供。
 */
export async function loadBoxes(): Promise<DesktopBox[]> {
  const database = await getDatabase();
  const rows = await database.select<Array<Record<string, unknown>>>(`
    SELECT id, title, x, y, width, height
    FROM ${APP_SETTINGS_STORAGE.tables.boxes}
    ORDER BY updated_at ASC
  `);

  return rows.map((row) => ({
    id: String(row.id),
    title: String(row.title),
    x: Number(row.x),
    y: Number(row.y),
    width: Number(row.width),
    height: Number(row.height),
  }));
}

/**
 * 读取 Box 和文件路径的关联表；Store 会基于它计算每个 Box 的项目列表和统计信息。
 */
export async function loadBoxItems(): Promise<DesktopBoxItem[]> {
  const database = await getDatabase();
  const rows = await database.select<Array<Record<string, unknown>>>(`
    SELECT box_id, item_path
    FROM ${APP_SETTINGS_STORAGE.tables.boxItems}
    ORDER BY updated_at ASC
  `);

  return rows.map((row) => ({
    boxId: String(row.box_id),
    itemPath: String(row.item_path),
  }));
}

/**
 * 保存单个 Box 状态；文件映射由 box_items 关联表独立维护，避免布局更新误写项目列表。
 */
export async function saveBox(box: DesktopBox): Promise<void> {
  const database = await getDatabase();

  await database.execute(
    `
      INSERT INTO ${APP_SETTINGS_STORAGE.tables.boxes} (id, title, x, y, width, height, updated_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        x = excluded.x,
        y = excluded.y,
        width = excluded.width,
        height = excluded.height,
        updated_at = excluded.updated_at
    `,
    [box.id, box.title, box.x, box.y, box.width, box.height, Date.now()],
  );
}

/**
 * 批量将桌面项目重新归入目标 Box；同一路径会先从其他 Box 移除，再写入目标关联。
 */
export async function assignBoxItems(boxId: string, itemPaths: string[]): Promise<void> {
  const database = await getDatabase();
  const uniqueItemPaths = Array.from(new Set(itemPaths)).filter(Boolean);

  if (uniqueItemPaths.length === 0) {
    return;
  }

  for (const itemPath of uniqueItemPaths) {
    await database.execute(
      `DELETE FROM ${APP_SETTINGS_STORAGE.tables.boxItems} WHERE item_path = $1`,
      [itemPath],
    );
    await database.execute(
      `
        INSERT INTO ${APP_SETTINGS_STORAGE.tables.boxItems} (box_id, item_path, updated_at)
        VALUES ($1, $2, $3)
        ON CONFLICT(box_id, item_path) DO UPDATE SET
          updated_at = excluded.updated_at
      `,
      [boxId, itemPath, Date.now()],
    );
  }
}

/**
 * 删除 Box 记录只影响 Dasktop 映射，不删除桌面文件本体。
 */
export async function deleteBoxRecord(boxId: string): Promise<void> {
  const database = await getDatabase();

  await database.execute(
    `DELETE FROM ${APP_SETTINGS_STORAGE.tables.boxItems} WHERE box_id = $1`,
    [boxId],
  );
  await database.execute(`DELETE FROM ${APP_SETTINGS_STORAGE.tables.boxes} WHERE id = $1`, [boxId]);
}

/**
 * 读取当前设置，并过滤掉旧版本残留或非法的设置值。
 */
export async function loadSettings(): Promise<AppSettings> {
  const database = await getDatabase();
  const rows = await database.select<Array<Record<string, unknown>>>(`
    SELECT key, value FROM ${APP_SETTINGS_STORAGE.tables.appSettings}
  `);
  const settings = { ...DEFAULT_APP_SETTINGS };
  const supportedKeys = new Set<keyof AppSettings>(SUPPORTED_APP_SETTING_KEYS);

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
  if (key === APP_SETTING_KEYS.theme) {
    return (isThemeMode(value) ? value : DEFAULT_APP_SETTINGS.theme) as AppSettings[Key];
  }

  if (
    key === APP_SETTING_KEYS.snapToEdges ||
    key === APP_SETTING_KEYS.showItemLabels ||
    key === APP_SETTING_KEYS.showShortcutArrow ||
    key === APP_SETTING_KEYS.doubleClickOpenItems
  ) {
    return (typeof value === "boolean" ? value : DEFAULT_APP_SETTINGS[key]) as AppSettings[Key];
  }

  if (key === APP_SETTING_KEYS.nameDisplayMode) {
    return (isDesktopNameDisplayMode(value)
      ? value
      : DEFAULT_APP_SETTINGS.nameDisplayMode) as AppSettings[Key];
  }

  if (key === APP_SETTING_KEYS.snapThreshold) {
    return (
      typeof value === "number" ? value : DEFAULT_APP_SETTINGS.snapThreshold
    ) as AppSettings[Key];
  }

  return DEFAULT_APP_SETTINGS[key];
}

/**
 * 主题枚举只允许当前三种值，避免历史状态继续污染 UI。
 */
function isThemeMode(value: unknown): value is AppSettings["theme"] {
  return THEME_MODES.some((themeMode) => themeMode === value);
}

/**
 * 文件名显示模式只接受当前版本提供的三个选项，避免历史字符串污染渲染逻辑。
 */
function isDesktopNameDisplayMode(value: unknown): value is AppSettings["nameDisplayMode"] {
  return DESKTOP_NAME_DISPLAY_MODES.some((displayMode) => displayMode === value);
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
      INSERT INTO ${APP_SETTINGS_STORAGE.tables.appSettings} (key, value, updated_at)
      VALUES ($1, $2, $3)
      ON CONFLICT(key) DO UPDATE SET
        value = excluded.value,
        updated_at = excluded.updated_at
    `,
    [key, JSON.stringify(value), Date.now()],
  );
}
