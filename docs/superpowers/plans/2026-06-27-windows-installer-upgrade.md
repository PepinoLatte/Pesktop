# Windows 安装器升级体验实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让 Windows 新版安装器优先呈现升级/更新语义，避免用户在正常版本更新时只能选择先卸载再安装。

**架构：** 使用 Tauri 官方 Windows bundler 配置固定安装范围、禁止降级和 MSI UpgradeCode，确保同一应用版本链可被安装器识别。中文安装器文案只描述升级、维护和降级边界，不接管 Tauri 生成的 NSIS/WiX 模板。

**技术栈：** Tauri 2.11 Windows bundler、NSIS、WiX/MSI、UTF-8 本地化文件。

---

### 任务 1：固定 Windows 安装器升级识别

**文件：**
- 修改：`src-tauri/tauri.conf.json`

- [ ] **步骤 1：写入稳定的 MSI UpgradeCode**

在 `bundle.windows.wix` 中加入当前 Tauri 计算出的默认 UpgradeCode：

```json
"upgradeCode": "254aa95b-a941-57f9-a27a-9fca09b603ee"
```

预期：后续版本即使调整产品名或打包环境，MSI 仍按同一产品线执行升级。

- [ ] **步骤 2：固定 NSIS 安装范围**

在 `bundle.windows.nsis` 中显式加入：

```json
"installMode": "currentUser"
```

预期：新版安装器继续写入 HKCU 安装记录，能识别旧的当前用户安装，不因安装范围不同出现重复安装或错误维护入口。

- [ ] **步骤 3：禁止降级覆盖安装**

在 `bundle.windows` 中加入：

```json
"allowDowngrades": false
```

预期：安装更旧版本时不伪装为升级，用户需要明确卸载后再回退。

### 任务 2：调整中文安装器升级语义

**文件：**
- 修改：`src-tauri/bundle/nsis/languages/SimpChinese.nsh`
- 修改：`src-tauri/bundle/wix/languages/zh-CN.wxl`

- [ ] **步骤 1：把旧版本检测文案改为升级安装**

将 NSIS 旧版本提示改成建议升级安装，并说明会保留配置和应用数据。

预期：用户安装新版时看到“升级/更新”语义，而不是被引导为先删除。

- [ ] **步骤 2：保留降级保护文案**

将 NSIS 和 WiX 的降级提示保持为“需先卸载当前版本后再安装旧版本”。

预期：升级和降级在文案上明确区分，避免误删数据。

### 任务 3：验证配置有效

**文件：**
- 读取：`node_modules/@tauri-apps/cli/config.schema.json`
- 运行：`pnpm build`
- 运行：`pnpm tauri inspect wix-upgrade-code`

- [ ] **步骤 1：验证 Tauri 配置字段在 schema 内**

确认 `upgradeCode`、`installMode`、`allowDowngrades` 都是 Tauri 2.11 schema 支持字段。

预期：配置解析不会因未知字段失败。

- [ ] **步骤 2：运行前端构建**

运行：

```bash
pnpm build
```

预期：`vue-tsc --noEmit && vite build` 成功。

- [ ] **步骤 3：复查 UpgradeCode**

运行：

```bash
pnpm tauri inspect wix-upgrade-code
```

预期：输出仍为 `254aa95b-a941-57f9-a27a-9fca09b603ee`，与配置固定值一致。
