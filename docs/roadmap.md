# Overclock-AI 实现规划

## Phase 1: CLI 编排核心 ← 当前

### 优先适配的 CLI 工具

| Tool | 厂商 | CLI 命令 | 安装方式 | 免费额度 |
|------|------|---------|---------|---------|
| **CodeBuddy** | Tencent | `codebuddy` | `npm install -g @tencent-ai/codebuddy-code` | 100K tokens/月 |
| **Kiro CLI** | AWS | `kiro-cli` | `curl -fsSL https://cli.kiro.dev/install \| bash` | 50 credits/月 (永久) |
| **Gemini CLI** | Google | `gemini` | `npm install -g @google/gemini-cli` | 免费额度依 API Key 而定 |

> 扩展计划：后续添加 Claude Code、Qwen Code CLI、Lingma 等适配器。

### Phase 1 核心功能与可靠性 (从纯编排走向 Harness)

我们将引入基于 Anthropic 最佳实践的 "Agent Harness" 理念，增加数据收集与严格验证，将系统从单纯的 "Toolbox" 升级为长周期可靠运行的体系。

### 实现步骤

| Step | 内容 | 状态 |
|------|------|------|
| 1.1 | 项目脚手架 (Rust workspace) | ✅ 已完成 |
| 1.2 | Core 数据模型 (task, role, workflow, context, event, config) | ✅ 已完成 |
| 1.3 | 适配器 Trait + CodeBuddy 适配器 | ✅ 已完成 |
| 1.4 | Kiro CLI + Gemini CLI 适配器 | ✅ 已完成 |
| 1.5 | CLI 命令行 (init, config, task, run, status) | ✅ 已完成 |
| 1.6 | 上下文同步机制 | ✅ 基础完成 |
| 1.7 | Agent Harness (证据驱动验证、重试控制、上下文压缩) | 🔲 待实现 |
| 1.8 | Telemetry & Data Collection (失败分析与性能收集) | 🔲 待实现 |
| 1.9 | TUI Monitor (终端实时监控) | 🔲 待实现 |
| 1.10 | 集成测试 + 文档 | 🔲 待实现 |

### 里程碑补充 (Harness & Reliability 集成)
由于长期自动化执行的需要，将在 Phase 1 加入以下关键能力：
1. **证据驱动验证**：所有任务完成前，必须向 Orchestrator 出具结构化证据（测试输出日志等），禁止简单的自行标注完成。
2. **自动化错误恢复**：建立问题分类模型（例如环境问题、依赖错误、代码等）。Orchestrator 能够重试调用或自动回退状态，减少人类干预。
3. **闭环的数据收集**：系统捕获所有任务的执行失败原因、使用的上下文字段，建立内置指标并存放到本地，服务于长期的规范升级与微调。


### 已验证

- `cargo build --workspace` — 零错误零警告
- `overclock-ai --help` — 命令行帮助正常
- `overclock-ai init` — 初始化项目结构 ✅
- `overclock-ai config agents/roles` — 配置展示 ✅
- `overclock-ai task create/list` — 任务管理 ✅
- `overclock-ai status` — CodeBuddy v1.100.0 ✅, Kiro CLI v1.26.2 ✅

## Phase 2: Web Kanban UI

- Axum REST API 服务器
- 前端看板 (React/Vue/Svelte — 待定)
- SSE/WebSocket 实时更新
- 角色配置 GUI
- 任务看板拖拽

## Phase 3: Mobile / Remote

- 移动端 Web App (PWA 或 React Native)
- 远程 SSH/Web Terminal 接入
- 推送通知 (任务完成/失败)
- 手机 App 查看开发进度

## 技术栈

| 层级 | 技术 |
|------|------|
| 核心引擎 | Rust (tokio async runtime) |
| CLI | clap 4 |
| 序列化 | serde + serde_json + toml |
| 日志 | tracing + tracing-subscriber |
| REST API (Phase 2) | Axum |
| 持久化 | 文件系统 JSON + SQLite (可选) |
| 辅助脚本 | Python, Bash |
