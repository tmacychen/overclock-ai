# AI CLI 适配器开发指南

## 概述

每个 AI CLI 适配器将一个具体的 CLI 工具（如 CodeBuddy、Kiro CLI）封装在统一的 `AgentAdapter` trait 后面。Orchestrator 通过此 trait 与所有 Agent 交互，保证了 Agent 的可互换性。

## Agent 隔离原则

```
Orchestrator 是唯一协调者：
- Agent 不知道其他 Agent 的存在
- Agent 只接收自己的 task description + curated context
- Agent 的输出通过 Orchestrator 汇总后传递给下游 Agent
- 所有上下文转换由 Adapter 在调用 CLI 前完成
```

## 实现 AgentAdapter trait

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// 人类可读的适配器名称
    fn name(&self) -> &str;

    /// 适配器类型标识符 (匹配 AgentConfig.agent_type)
    fn agent_type(&self) -> &str;

    /// 检查 CLI 工具是否已安装、已认证、可用
    async fn health_check(&self) -> HealthStatus;

    /// 执行一个任务
    /// 适配器负责：
    /// 1. 用 task description 构建 CLI 命令
    /// 2. 将共享上下文转换为 CLI 原生格式并注入
    /// 3. 作为子进程运行 CLI
    /// 4. 解析输出为统一的 TaskOutput
    async fn execute_task(
        &self,
        task: &Task,
        context: &SharedContext,
        config: &AgentConfig,
    ) -> Result<TaskOutput>;

    /// 查询额度/用量信息 (可选)
    async fn quota_info(&self, config: &AgentConfig) -> Result<Option<QuotaInfo>>;
}
```

## 已实现的适配器

### CodeBuddy (`codebuddy.rs`)

| 属性 | 值 |
|------|-----|
| 安装命令 | `npm install -g @tencent-ai/codebuddy-code` |
| CLI 命令 | `/opt/homebrew/lib/node_modules/@tencent-ai/codebuddy-code/bin/codebuddy` |
| 健康检查 | `codebuddy --version` |
| 上下文注入 | Prompt 前置拼接 |
| 输出解析 | stdout 原文 |

> **注意**：如果安装了 CodeBuddy IDE，全局的 `codebuddy` 命令可能会冲突。建议在配置中使用完整路径。

### Kiro CLI (`kiro.rs`)

| 属性 | 值 |
|------|-----|
| CLI 命令 | `kiro-cli chat --message <prompt>` |
| 健康检查 | `kiro-cli --version` |
| 上下文注入 | Prompt 前置拼接 |
| 输出解析 | stdout 原文 |

### Gemini CLI (`gemini.rs`)

| 属性 | 值 |
|------|-----|
| CLI 命令 | `gemini run <prompt>` |
| 健康检查 | `gemini --version` |
| 上下文注入 | Prompt 前置拼接 |
| 输出解析 | stdout 原文 |

## 添加新的适配器

1. 在 `crates/overclock-adapters/src/` 下创建新模块文件 (如 `claude_code.rs`)
2. 实现 `AgentAdapter` trait
3. 在 `lib.rs` 中注册模块
4. 在 CLI 的 `status.rs` 和 `run.rs` 中添加 match 分支
5. 在 `overclock-ai.toml` 中添加 agent 配置

### 模板

```rust
use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use crate::adapter_trait::{AgentAdapter, HealthStatus, TaskOutput};

pub struct MyAdapter;

impl MyAdapter {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl AgentAdapter for MyAdapter {
    fn name(&self) -> &str { "My CLI Tool" }
    fn agent_type(&self) -> &str { "my-cli" }

    async fn health_check(&self) -> HealthStatus {
        match tokio::process::Command::new("my-cli")
            .arg("--version").output().await
        {
            Ok(output) if output.status.success() => {
                HealthStatus::Ready {
                    version: String::from_utf8_lossy(&output.stdout).trim().to_string()
                }
            }
            _ => HealthStatus::NotInstalled {
                reason: "my-cli not found".into()
            },
        }
    }

    async fn execute_task(
        &self, task: &Task, context: &SharedContext, config: &AgentConfig,
    ) -> anyhow::Result<TaskOutput> {
        let prompt = format!("{}\n\n# Task: {}\n{}", 
            context.to_prompt_context(), task.title, task.description);
        
        let output = tokio::process::Command::new(
            config.binary.as_deref().unwrap_or("my-cli"))
            .arg("run").arg(&prompt)
            .current_dir(&context.workspace_root)
            .output().await?;

        Ok(TaskOutput {
            success: output.status.success(),
            summary: String::from_utf8_lossy(&output.stdout)
                .lines().take(5).collect::<Vec<_>>().join("\n"),
            modified_files: vec![],
            artifacts: vec![],
            raw_output: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code(),
        })
    }
}
```
