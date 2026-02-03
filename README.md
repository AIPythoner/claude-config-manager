# Claude Config Manager

一个用于管理多种 AI 工具配置的桌面应用，支持 Claude Code、Gemini CLI、Codex CLI，通过修改配置文件和环境变量来切换不同的 API 配置。

## 功能特性

- **多平台支持** - 支持 Claude Code、Gemini CLI、Codex CLI 三种 AI 工具
- **多配置管理** - 支持添加、编辑、删除多个配置
- **一键切换** - 点击配置项即可激活，自动设置对应工具的配置
- **流畅体验** - 配置切换时显示优雅的loading动画，支持窗口拖动
- **持久化存储** - 配置保存在本地，重启后依然有效
- **简洁界面** - 小窗口设计，轻快明亮配色，卡片式配置展示

## 支持的 AI 工具

| 工具 | 配置方式 | 说明 |
|------|----------|------|
| **Claude Code** | 环境变量 | 设置 `ANTHROPIC_AUTH_TOKEN` 和 `ANTHROPIC_BASE_URL` |
| **Gemini CLI** | 配置文件 | 写入 `~/.opencode/config.json` |
| **Codex CLI** | 配置文件 | 写入 `~/.codex/auth.json` |

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
│   │   ├── lib.rs           # 核心逻辑（配置存储、环境变量操作）
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

1. **配置存储**：配置以 JSON 格式保存在 `%APPDATA%/claude-config-manager/configs.json`

2. **环境变量修改**：通过 Windows 注册表 API 修改用户级环境变量
   - 写入位置：`HKEY_CURRENT_USER\Environment`
   - 修改后新打开的终端会自动读取新值

3. **激活配置**：点击配置项时，程序会将对应的 Token 和 URL 写入系统环境变量

## 使用说明

1. 启动应用后，点击右上角「添加」按钮
2. 选择配置类型（Claude / Gemini / Codex）
3. 填写配置信息：
   - **配置名称**：便于识别的名称，如"个人账户"、"公司账户"
   - **API Key**：对应工具的 API 密钥
   - **Base URL**：可选，自定义 API 地址
4. 点击「添加」保存配置
5. 点击配置项即可激活，绿色标记表示当前激活的配置
6. 根据不同工具类型，配置会自动写入对应位置

## 常见问题

### Q: 修改配置后 Claude Code 没有生效？
A: 环境变量修改后，需要重新打开终端或重启 Claude Code 才能读取新值。

### Q: 如何完全删除环境变量？
A: 删除当前激活的配置时，程序会自动清除相关环境变量。

### Q: 配置文件在哪里？
A: `%APPDATA%/claude-config-manager/configs.json`

## 技术栈

- **前端**：TypeScript + Vite
- **后端**：Rust + Tauri 2.0
- **UI**：原生 CSS（无框架依赖）

## License

MIT
