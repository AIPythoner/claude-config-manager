# Claude Config Manager

一个用于管理多种 AI 工具配置的桌面应用，支持 Claude Code、Gemini CLI、Codex CLI、OpenCode，全部通过修改各工具自己的配置文件来切换 API 配置，无需重启终端、无污染系统环境变量。

## 功能特性

- **多平台支持** - 支持 Claude Code、Gemini CLI、Codex CLI、OpenCode 四种 AI 工具
- **多配置管理** - 支持添加、编辑、删除多个配置
- **一键切换** - 点击配置项即可激活，自动写入对应工具的配置文件
- **不污染系统** - 不修改 Windows 注册表/系统环境变量，仅写入工具自身配置目录
- **流畅体验** - 配置切换时显示优雅的 loading 动画，支持窗口拖动
- **持久化存储** - 配置保存在本地，重启后依然有效
- **简洁界面** - 小窗口设计，轻快明亮配色，卡片式配置展示

## 支持的 AI 工具

| 工具 | 写入位置 | 说明 |
|------|----------|------|
| **Claude Code** | `~/.claude/settings.json` | 合并写入 `env.ANTHROPIC_AUTH_TOKEN` 与 `env.ANTHROPIC_BASE_URL`，仅修改这两个键，其它字段保持不变 |
| **Gemini CLI** | `~/.gemini/.env` | 合并写入 `GEMINI_API_KEY` 与 `GOOGLE_GEMINI_BASE_URL`，仅替换这两个键，其它行保持不变 |
| **Codex CLI** | `~/.codex/auth.json` 与 `~/.codex/config.toml` | 写入 OpenAI API Key 和模型 provider 配置 |
| **OpenCode** | `~/.config/opencode/opencode.json` | 在 OpenCode 标签页选择已有 Claude/Gemini/Codex 配置，一键合并写入 provider |

## 环境要求

- Windows 10/11
- Node.js 18+
- Rust 1.70+
- npm 或 pnpm

## 安装依赖

```bash
# 进入项目目录
cd claude-config-manager

# 安装前端依赖
npm install
```

## 开发模式

```bash
npm run tauri dev
```

这会同时启动 Vite 开发服务器和 Tauri 应用窗口，支持热重载。

## 打包构建

### 构建安装程序（推荐）

```bash
npm run tauri build
```

构建完成后，安装程序位于：
```
src-tauri/target/release/bundle/nsis/Claude Config Manager_1.0.0_x64-setup.exe
```

### 仅构建 EXE

如果只需要单独的 exe 文件（无需安装），构建后可在以下位置找到：
```
src-tauri/target/release/claude-config-manager.exe
```

> 注意：单独的 exe 需要 WebView2 运行时支持。安装程序版本会自动处理依赖。

## 项目结构

```
claude-config-manager/
├── src/                      # 前端源码
│   ├── main.ts              # 主逻辑（配置管理、UI 交互）
│   └── styles.css           # 样式文件
├── src-tauri/               # Rust 后端
│   ├── src/
│   │   ├── lib.rs           # 核心逻辑（配置存储、目标工具配置文件读写）
│   │   └── main.rs          # 应用入口
│   ├── icons/               # 应用图标
│   ├── Cargo.toml           # Rust 依赖配置
│   └── tauri.conf.json      # Tauri 配置
├── index.html               # HTML 入口
├── package.json             # Node 依赖配置
├── vite.config.ts           # Vite 配置
└── README.md                # 本文档
```

## 工作原理

1. **本地配置存储**：用户保存的配置以 JSON 格式存放在 `%APPDATA%/claude-config-manager/configs.json`

2. **激活配置**：点击配置项时，程序根据配置类型把对应字段写入目标工具的配置文件
   - **Claude Code**：读取 `~/.claude/settings.json`，仅合并更新 `env` 字段中的 `ANTHROPIC_AUTH_TOKEN` 与 `ANTHROPIC_BASE_URL`，其它字段（permissions、statusLine、enabledPlugins 等）保持不变
   - **Gemini CLI**：读取 `~/.gemini/.env`，按行替换 `GEMINI_API_KEY` 与 `GOOGLE_GEMINI_BASE_URL`，注释和其它键保持不变
   - **Codex CLI**：写入 `~/.codex/auth.json`（API Key）和 `~/.codex/config.toml`（provider 配置）
   - **OpenCode**：合并更新 `~/.config/opencode/opencode.json` 中对应 provider 的 `apiKey` / `baseURL`

3. **取消激活**：删除或停用当前激活的配置时，程序会从对应配置文件中移除写入的键，保持其它内容不变

## 使用说明

1. 启动应用后，点击右上角「添加」按钮
2. 选择配置类型（Claude / Gemini / Codex）
3. 填写配置信息：
   - **配置名称**：便于识别的名称，如"个人账户"、"公司账户"
   - **API Key**：对应工具的 API 密钥
   - **Base URL**：可选，自定义 API 地址
4. 点击「添加」保存配置
5. 点击配置项即可激活，绿色标记表示当前激活的配置
6. 切换 OpenCode 标签可以从已有 Claude/Gemini/Codex 配置中选择，一键写入 OpenCode

## 常见问题

### Q: 修改配置后是否需要重启终端？
A: 不需要。Claude Code、Gemini CLI 都会在每次启动时重新读取自己的配置文件 / `.env`，下次启动新进程即可生效，无需重启系统或终端。

### Q: 是否会修改我的 Windows 系统环境变量？
A: **不会**。本工具完全不再操作 Windows 注册表 (`HKCU\Environment`)，只在用户家目录下的工具自有配置目录中写入键值。

### Q: 之前用旧版本设置过的 Windows 环境变量怎么办？
A: 旧版本会写入 `HKCU\Environment`。升级后本工具不再清理它们，可手动通过 “系统属性 → 环境变量” 删除 `ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_BASE_URL`、`GEMINI_API_KEY`、`GOOGLE_GEMINI_BASE_URL` 等键，避免与新方式冲突。

### Q: 删除当前激活的配置会发生什么？
A: 程序会从对应工具的配置文件中移除这次写入的键（例如 `~/.claude/settings.json` 的 `env.ANTHROPIC_AUTH_TOKEN`），其它字段保持原样。

### Q: 应用自身的配置文件在哪里？
A: `%APPDATA%/claude-config-manager/configs.json`

## 技术栈

- **前端**：TypeScript + Vite
- **后端**：Rust + Tauri 2.0
- **UI**：原生 CSS（无框架依赖）

## License

MIT
