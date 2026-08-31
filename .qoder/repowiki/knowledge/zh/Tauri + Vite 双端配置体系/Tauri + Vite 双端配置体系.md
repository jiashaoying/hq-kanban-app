---
kind: configuration_system
name: Tauri + Vite 双端配置体系
category: configuration_system
scope:
    - '**'
source_files:
    - src-tauri/tauri.conf.json
    - src-tauri/src/lib.rs
    - src-tauri/src/main.rs
    - src-tauri/Cargo.toml
    - vite.config.ts
    - package.json
    - src-tauri/capabilities/default.json
---

## 1. 使用的系统与框架

本项目采用 **Tauri v2** 作为桌面应用宿主，前端基于 **Vue 3 + Vite**，后端为 Rust。配置系统由 Tauri 与 Vite 两套独立但协作的配置机制组成：
- Tauri 侧通过 `src-tauri/tauri.conf.json` 管理应用元信息、窗口、打包与构建行为。
- Vite 侧通过 `vite.config.ts` 管理开发服务器、插件与 HMR 行为。
- Rust 侧依赖声明集中在 `src-tauri/Cargo.toml`，无运行时配置文件。

项目没有使用 `.env`、`.yaml`、`.toml`（除 Cargo 清单外）或环境变量注入的运行时配置加载逻辑，所有可配置项均以静态 JSON / TS 文件形式声明。

## 2. 关键文件

- `src-tauri/tauri.conf.json` — Tauri 应用主配置（产品名、版本、标识符、窗口、安全策略、bundle 图标等）。
- `src-tauri/src/lib.rs` — Tauri 启动入口，注册命令处理器并调用 `tauri::generate_context!()` 读取上述 JSON 上下文。
- `src-tauri/src/main.rs` — 仅做 Windows 子系统开关与转发到 `hq_kanban_app_lib::run()`，不含配置逻辑。
- `src-tauri/Cargo.toml` — Rust crate 元数据与依赖声明（含 `tauri-build`、`reqwest`、`tokio` 等）。
- `package.json` — Node 工程脚本（`dev`/`build`/`tauri`），定义前后端构建流程。
- `vite.config.ts` — Vite 开发服务器端口固定为 `1420`，HMR 端口 `1421`，通过 `TAURI_DEV_HOST` 环境变量控制 host/HMR 行为。
- `src-tauri/capabilities/default.json` — Tauri v2 能力清单（ACL 权限声明）。

## 3. 架构与设计约定

### 3.1 构建期配置 vs 运行期配置
- **构建期配置**：`tauri.conf.json` 中的 `build.beforeDevCommand` = `pnpm dev`、`build.beforeBuildCommand` = `pnpm build`、`build.frontendDist` = `../dist`，将 Vite 产物直接嵌入 Tauri 包中；`app.windows` 定义窗口尺寸、标题、是否装饰等 UI 行为。
- **运行期配置**：当前代码未实现任何运行时配置加载（如从文件或环境变量读取 API Key、URL 等）。市场数据抓取逻辑硬编码在 `market.rs` 中（腾讯财经接口），Rust 侧无配置模块。

### 3.2 前后端配置联动
- Tauri 通过 `tauri::generate_context!()` 在编译时把 `tauri.conf.json` 的内容注入到二进制中，Rust 侧无需手动解析 JSON。
- Vite 开发服务器端口 `1420` 与 `tauri.conf.json` 中 `build.devUrl` 保持一致，确保 Tauri 能正确代理到前端。
- `vite.config.ts` 通过 `process.env.TAURI_DEV_HOST` 动态决定 `server.host` 和 HMR 配置，这是唯一跨进程的环境变量交互点。

### 3.3 能力与权限模型
- Tauri v2 使用 `capabilities/default.json` 声明 IPC 能力，而非旧版 `permissions` 列表，体现向声明式 ACL 迁移。

## 4. 约定与约束

- **窗口配置集中化**：所有窗口相关属性（宽高、标题、居中、可调整大小、无边框、置顶）均集中在 `tauri.conf.json` 的 `app.windows` 数组中，未在 Rust 代码中二次声明。
- **安全策略显式关闭 CSP**：`app.security.csp` 设为 `null`，意味着不启用内容安全策略（适合本地桌面场景，但需开发者自行保证资源来源安全）。
- **端口固定约定**：Vite 使用 `strictPort: true` 强制占用 `1420`，Tauri 也期望该端口，避免开发时端口冲突导致连接失败。
- **忽略 Rust 源码热更新**：Vite watch 配置显式 `ignored: ["**/src-tauri/**"]`，防止修改 Rust 代码触发前端重建。
- **无运行时配置加载器**：项目中不存在 `Config` struct、`load_config` 函数、`.env` 解析、JSON/YAML 配置文件的读取逻辑；所有外部依赖（如 HTTP 请求目标）以硬编码形式存在。
- **依赖版本锁定**：Cargo 与 pnpm lockfile 分别锁定 Rust 与 Node 依赖版本，确保构建可重现。

## 5. 缺失与扩展点

当前配置体系仅覆盖“应用元数据 + 构建/打包”层面，尚未包含：
- 运行时配置（如 API 地址、轮询间隔、主题偏好）持久化。
- 环境区分（dev/staging/prod）的配置切换。
- 用户级配置存储（可通过 Tauri 的 `fs` 能力或平台原生配置目录扩展）。
- Rust 侧的 `config` crate 或 `dotenv` 集成。

若后续需要引入运行时配置，建议沿用现有模式：新增 JSON/TOML 配置文件并通过 Tauri 命令暴露读写接口，保持与 Vue 前端的 IPC 通信一致性。