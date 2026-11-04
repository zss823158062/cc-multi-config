# Claude 多实例配置启动器 (Claude Multi-Instance Launcher)

面向 **Claude Code** 的跨平台桌面启动器，基于 **Tauri 2 + Vue 3 + TypeScript + Rust** 构建。

## 🌟 核心价值与业务规则

1. **分组独立**：每个模型分组对应独立的配置文件 `my-<slug>.json`（如 `my-opus.json`、`my-sonnet.json`）。
2. **基础配置不污染**：严格不修改用户原有的 `~/.claude/settings.json`；新建分组时只读取并复制用户自定义配置（如 `permissions`、`hooks` 等），并自动剥离连接与模型覆盖字段。
3. **节点共享与复用**：统一管理 API 中转站地址、认证方式和模型映射，多个模型分组可随时切换复用同一个节点。
4. **安全与原子操作**：
   - 配置修改前自动生成时间戳备份（保留最近 10 次修改历史，支持一键还原）。
   - 采用临时文件写入 + 原子重命名（Atomic Replacement），避免写入中断导致文件损坏。
   - 差异对比（Diff Preview）与脱敏日志，Token 凭据均以星号脱敏。
5. **跨平台终端集成**：
   - **Windows**：自动适配 Windows Terminal (`wt.exe`)，兼容 PowerShell 与 CMD。
   - **macOS**：支持系统终端 Terminal.app 与 iTerm2。
   - 以 `--settings <绝对路径>` 传参，支持包含中文、空格的工作路径。

---

## 🏗️ 架构与模块设计

```text
┌───────────────────────────────────────────────┐
│ Vue 3 前端 (TypeScript + Dark Modern Theme)    │
│ 分组切换 | 节点测速管理 | 工作目录选择 | 配置差异预览   │
└───────────────────────┬───────────────────────┘
                        │ Tauri 2 IPC Commands
┌───────────────────────▼───────────────────────┐
│ Rust 核心业务层                                │
│ ├── PathResolver (路径解析与安全检查、Slug 校验) │
│ ├── GroupService (分组 CRUD、原子创建)         │
│ ├── NodeService (中转节点管理与跨组同步)         │
│ ├── SettingsMerger (白名单字段合并与 Diff 生成) │
│ ├── BackupService (历史备份与原子写入)         │
│ ├── NodeTester (网络/HTTP 测速与状态分类)      │
│ └── LaunchService (Claude 检测与终端进程调度)  │
└───────────────────────────────────────────────┘
```

---

## 🚀 快速开始

### 前置要求
- **Node.js**: v18+ (推荐 v20+)
- **pnpm**: v9+ / v10+
- **Rust**: 1.80+ (推荐 1.85+)
- **Claude Code**: 安装于 PATH 或用户目录

### 1. 安装前端依赖
```bash
pnpm install
```

### 2. 启动桌面开发调试模式
```bash
pnpm tauri dev
```

### 3. 构建发布安装包
```bash
pnpm tauri build
```
编译产物位于 `src-tauri/target/release/`。

### 4. 运行现有 Debug 可执行文件
已预先完成编译，可直接双击运行：
```powershell
.\src-tauri\target\debug\claude-launcher.exe
```

---

## 🧪 单元测试

后端核心算法均包含自动化单元测试（Slug 安全清洗、托管字段剥离保留自定义配置、节点合并模型映射、Token 差异脱敏）：

```bash
cd src-tauri
cargo test
```

测试结果：
- `test_slug_sanitization`：通过（支持特殊符号转短横线、拦截 Windows 保留名如 CON/PRN）
- `test_clean_managed_fields_preserves_user_settings`：通过（保留 permissions, hooks 等配置）
- `test_merge_node_to_settings`：通过（正确写入 ANTHROPIC_BASE_URL、模型映射与 Token）
- `test_preview_diff_masks_token`：通过（敏感信息星号脱敏）

---

## 📁 目录规范

| 资源 | Windows 路径 | macOS 路径 |
| --- | --- | --- |
| Claude 配置目录 | `%USERPROFILE%\.claude\` | `~/.claude/` |
| 基础配置 (只读) | `%USERPROFILE%\.claude\settings.json` | `~/.claude/settings.json` |
| 分组独立配置 | `%USERPROFILE%\.claude\my-<slug>.json` | `~/.claude/my-<slug>.json` |
| 启动器应用数据 | `%APPDATA%\ClaudeLauncher\` | `~/Library/Application Support/ClaudeLauncher/` |
| 自动备份目录 | `%APPDATA%\ClaudeLauncher\backups\` | `~/Library/Application Support/ClaudeLauncher/backups/` |
