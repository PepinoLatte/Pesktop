import packageInfo from "../../../package.json";

/**
 * 包名使用 npm 小写命名，界面展示时恢复产品名首字母大写，避免再维护一份独立名称常量。
 */
function resolvePackageDisplayName(packageName: string): string {
  if (!packageName) {
    return "Dasktop";
  }

  return `${packageName.slice(0, 1).toUpperCase()}${packageName.slice(1)}`;
}

/**
 * 应用展示元信息统一从包元数据派生，避免设置侧栏、关于面板和发布版本号各自硬编码后不同步。
 */
export const APP_META = {
  author: packageInfo.author,
  displayName: resolvePackageDisplayName(packageInfo.name),
  packageName: packageInfo.name,
  poweredBy: "Codex",
  version: packageInfo.version,
} as const;
