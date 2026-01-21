# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

sysMaster 是一个用 Rust 实现的新一代 1 号进程（init 系统），旨在替代 systemd。采用 **1+1+N 架构**：init（极简 PID 1）+ core（核心服务管理）+ exts（可替换扩展组件）。

核心目标：永不宕机、快速启动、极简镜像、内存安全。

## 构建和测试

### 环境准备
```bash
# 首次构建：安装依赖、设置 pre-commit hooks
sh ./build.sh
```

### 常用命令

```bash
# 完整构建（包含格式检查、编译、测试）
sh ci/01-pre-commit.sh

# 格式检查
cargo fmt -v --all -- --check -v

# Lint 检查（严格模式，任何 warning 视为错误）
cargo clippy -vvv --all-targets --features "default" --all -- -Dwarnings

# 自动修复代码问题
cargo fix -v --broken-code --all-targets --all --allow-dirty --allow-staged

# 构建
cargo build --all --features "default" -v

# 运行测试（单线程、完整 backtrace、显示输出）
RUST_BACKTRACE=full cargo test --all-targets --all -v -- --nocapture --show-output --test-threads=1
```

### Rust 版本
项目基于 Rust 1.57 构建。pre-commit 流程会自动设置正确的 Rust 版本。

### Pre-commit Hooks
每次 git commit 会自动执行：
- 拼写检查（codespell）
- 构建检查
- Clippy lint
- 代码格式检查
- 完整测试套件
- 临时文件清理

## 核心架构

### 1+1+N 架构层次

```
init/                    # PID 1，千行级极简代码
    └── main.rs         # 信号处理、故障恢复、调用 sysmaster-core

core/                   # 核心组件（sysmaster-core）
    ├── sysmaster/      # 主管理器：事件循环、job 引擎、可靠性框架
    ├── libcore/        # 内部库：Unit trait、错误处理、序列化
    ├── sctl/           # CLI 工具（类似 systemctl）
    └── coms/           # Unit 类型插件（组件）
        ├── service/    # 服务管理（进程生命周期）
        ├── socket/     # Socket 激活
        ├── target/     # 目标/运行级别
        ├── mount/      # 挂载点管理
        └── timer/      # 定时器

libs/                   # 公共库（跨模块共享）
    ├── event/          # 事件驱动引擎（基于 mio）
    ├── log/            # 日志系统
    ├── cgroup/         # cgroup 控制器
    ├── basic/          # 基础工具（feature 驱动模块化）
    ├── cmdproto/       # 命令协议
    └── ...             # 其他专用库

exts/                   # 扩展组件（可独立替换）
    ├── devmaster/      # 设备管理（替代 udev）
    ├── random_seed/    # 随机数种子
    ├── fstab/          # /etc/fstab 解析
    └── ...             # 其他扩展
```

### 核心概念

#### Unit 系统
- **Unit**：系统服务的基本配置单元（类似 systemd）
- **SubUnit**：具体类型（service、socket、target、mount、timer 等）
- **Job**：Unit 状态迁移的最小事务单元，支持回滚
- **UnitManager**：管理所有 Unit 生命周期、依赖关系

配置文件路径优先级：
1. `/etc/sysmaster/system` （最高）
2. `/run/sysmaster/system`
3. `/usr/lib/sysmaster/system` （最低）

#### 事件驱动架构
- 基于 mio 的事件循环
- 事件驱动引擎接收外部事件（信号、socket、notify）
- 事件触发 Unit 状态机迁移
- Job 引擎保证状态迁移的原子性

#### 可靠性框架
- 故障检查点动态注入
- 状态外置（DataStore）
- Savepoint 技术实现秒级自愈
- init 进程盲等待并巡检 core 状态

### 关键数据流

启动流程：
```
1. init (PID 1) 启动
   - 设置信号处理（忽略所有信号）
   - 安装崩溃处理器（SIGSEGV, SIGILL 等）
   - 创建 sysmaster-core 运行时
   - 监听 core 心跳，故障时重启 core

2. sysmaster-core 启动
   - 忽略所有信号，注册感兴趣的信号
   - 重挂载 / 为读写（日志需要）
   - 初始化日志系统
   - 设置 cgroup 控制器
   - 加载配置文件
   - 注册插件（各 SubUnit 类型）
   - 启动 UnitManager
   - 进入主事件循环
```

Unit 执行流程：
```
外部命令/事件 → UnitManager → 创建 Job → JobEngine 执行
                                    ↓
                            Unit 状态机迁移
                                    ↓
                            SubUnit 具体动作
                                    ↓
                            事务提交/回滚
```

## 插件机制

### Unit 类型插件
位于 `core/coms/`，每个目录实现一种 Unit 类型：

```rust
// 每个插件必须实现 Unit trait
pub trait Unit {
    // Unit 生命周期方法
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn reload(&self) -> Result<()>;
    // ... 其他接口
}
```

插件通过 feature flags 编译进 sysmaster，在启动时动态注册。

### Feature-driven 模块化
`libs/basic/` 使用 feature flags 实现模块化：
- 每个 feature 对应一个模块（如 `feature = "signal"`）
- 按需启用，减少二进制大小
- 支持场景化裁剪

## 关键设计模式

### 错误处理
- 使用 `core::error::Error` 枚举统一错误类型
- `Result<T>` 风格错误传播
- 严格模式：`#![deny(warnings)]`、`#![deny(missing_docs)]`、`#![deny(clippy::all)]`

### 内存管理
- Rust 所有权系统保证内存安全
- 极少使用 `unsafe`（主要在 FFI 和信号处理）
- `Rc<RefCell<T>>` 用于共享可变状态（单线程环境）

### 依赖管理
- Workspace 结构，每个目录一个 crate
- lib crate 带前缀 `lib`，daemon crate 以 `d` 结尾
- 顶层 `Cargo.toml` 定义 workspace members

## 代码组织规范

### 命名约定
- lib crate: `libs/event`、`libs/basic`
- bin crate: `core/sysmaster`、`exts/random_seed`
- daemon crate: `exts/devmaster`

### 文档要求
- 所有 Rust 文件必须包含 `#![deny(missing_docs)]`
- 公共 API 必须有文档注释
- 模块级注释说明职责和依赖关系

### 代码质量门槛
- Pre-commit 通过是提交的前提
- 任何 warning 视为错误（`-Dwarnings`）
- 必须通过完整测试套件
- 中文字符串需通过 codespell 检查（有忽略列表）

## 特殊注意事项

### 信号处理
- init 进程：忽略所有信号，除了 SIGKILL/SIGSTOP
- sysmaster-core：显式忽略所有信号，然后注册感兴趣的信号
- 崩溃信号（SIGSEGV 等）触发 panic，由可靠性框架恢复

### 测试
- 单线程执行（`--test-threads=1`）
- 完整 backtrace（`RUST_BACKTRACE=full`）
- 显示所有输出（`--nocapture --show-output`）
- 测试前清理临时文件（`rm -rf target/*/reliability/`）

### 构建优化
- Dev profile: `opt-level = 'z'`（优化大小），支持增量编译
- Release profile: `opt-level = 'z'`，LTO 启用，panic = abort
- 单代码生成单元（`codegen-units = 1`）最大化优化

### 兼容性
- 提供 systemd 配置文件兼容（TOML 格式）
- 提供 `sctl` 命令行工具（类似 `systemctl`）
- 支持在容器、虚机、裸机场景运行

## 相关文档

- 架构设计：`docs/design/00-sysmaster_architecture.md`
- Unit 管理：`docs/design/cores/01-unitManger.md`
- 使用指南：`docs/zh/sysmaster_usage.md`
- 故障排查：`docs/zh/troubleshooting.md`
- API 文档：`docs/man/` 目录
