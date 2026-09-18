import { invoke } from "@tauri-apps/api/core";

/**
 * 外置前端模式走自定义协议的主机名；与 Rust 侧 EXTERNAL_FRONTEND_SCHEME 保持一致
 */
const EXTERNAL_FRONTEND_ORIGIN = "http://daskhot.localhost";

let externalFrontendPromise: Promise<boolean> | null = null;

/**
 * 查询当前是否启用外置前端资源；结果进程内缓存，运行期不会切换模式
 */
export function isExternalFrontend(): Promise<boolean> {
  externalFrontendPromise ??= invoke<boolean>("is_external_frontend").catch(() => false);
  return externalFrontendPromise;
}

/**
 * 按当前模式解析动态窗口的加载地址：
 * 外置模式返回 daskhot 协议绝对地址（从 exe 旁 dist 实时读取），内嵌模式原样返回相对路径
 */
export async function resolveAppWindowUrl(path: string): Promise<string> {
  if (await isExternalFrontend()) {
    return `${EXTERNAL_FRONTEND_ORIGIN}${path}`;
  }

  return path;
}
