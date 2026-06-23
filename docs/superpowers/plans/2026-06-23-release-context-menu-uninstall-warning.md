# 发行版右键菜单与卸载提示文案实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 发行版隐藏 WebView 原生右键菜单，并在卸载删除配置选项中提示 Dasktop 创建的 Box 文件夹及其中内容可能被删除。

**架构：** 前端入口在生产环境注册全局 `contextmenu` 捕获监听器，只阻止浏览器默认菜单，不阻断组件右键逻辑。NSIS 中文语言文件更新删除应用数据选项的风险提示。

**技术栈：** Vue 3、Vite、Tauri 2、NSIS 自定义语言文件。

---

## 文件结构

- 修改：`src/app/main.ts`，新增生产环境右键默认菜单兜底。
- 修改：`src-tauri/bundle/nsis/languages/SimpChinese.nsh`，更新卸载删除配置文案。

## 任务 1：隐藏发行版 WebView 原生右键菜单

**文件：**
- 修改：`src/app/main.ts`

- [x] **步骤 1：新增生产环境兜底函数**

在应用挂载前加入：

```ts
/**
 * 生产包面向普通用户，WebView 默认菜单会暴露浏览器式操作并打断桌面工具体验。
 * 这里只阻止默认菜单，不阻断事件传播，保证文件项右键仍可触发 Windows Shell 菜单。
 */
function suppressNativeContextMenuInProduction(): void {
  if (!import.meta.env.PROD) {
    return;
  }

  window.addEventListener(
    "contextmenu",
    (event) => {
      event.preventDefault();
    },
    { capture: true },
  );
}
```

- [x] **步骤 2：在挂载前调用**

在 `createApp(App).use(createPinia()).mount("#app")` 之前调用：

```ts
suppressNativeContextMenuInProduction();
```

## 任务 2：更新卸载删除配置提示

**文件：**
- 修改：`src-tauri/bundle/nsis/languages/SimpChinese.nsh`

- [x] **步骤 1：更新文件头注释**

把旧的“不会删除真实桌面文件”边界说明改为强调删除配置时的文件夹风险。

- [x] **步骤 2：更新 `deleteAppData` 文案**

将文案改为：

```nsis
LangString deleteAppData ${LANG_SIMPCHINESE} "删除 Dasktop 配置和应用数据（含软件创建的 Box 文件夹；其中内容可能被删除，请提前转移需要保留的文件）"
```

## 任务 3：验证

**文件：**
- 检查：`src/app/main.ts`
- 检查：`src-tauri/bundle/nsis/languages/SimpChinese.nsh`

- [x] **步骤 1：运行前端构建**

运行：

```bash
pnpm build
```

预期：`vue-tsc --noEmit` 和 `vite build` 通过。
