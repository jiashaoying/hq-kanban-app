---
kind: build_system
name: Tauri + Vite 双端构建与打包体系
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - vite.config.ts
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/tauri.conf.json
    - pnpm-lock.yaml
    - Cargo.lock
---

## 1. 使用的系统/方法

本项目采用 **Tauri v2 + Vue 3 + Vite** 的混合构建方案：前端使用 Vite（Vue + TypeScript + Tailwind CSS）进行开发与静态资源构建，后端使用 Rust/Cargo 作为 Tauri 原生层。最终通过 `tauri` CLI 将前端产物与 Rust 二进制捆绑为跨平台桌面应用。

- 包管理器：**pnpm**（由 `pnpm-lock.yaml` 锁定版本 11.24.0）
- 前端构建工具：**Vite 8** + `@vitejs/plugin-vue` + `vue-tsc` 类型检查
- 原生层构建：**Cargo**（Rust edition 2021），通过 `tauri-build` 在编译期生成 Tauri 相关代码
- 应用打包：**Tauri CLI**（`@tauri-apps/cli` v2.11.4），负责将前端 `dist/` 与 Rust 二进制打包成各平台安装包

## 2. 关键文件与位置

| 文件 | 作用 |
|---|---|
| `package.json` | 定义前端脚本 `dev` / `build` / `preview` / `tauri`，声明依赖与版本 |
| `vite.config.ts` | 配置 Vite 开发服务器固定端口 1420、HMR、忽略 `src-tauri` 监听 |
| `src-tauri/Cargo.toml` | Rust crate 元数据、依赖（`tauri`、`reqwest`、`tokio`、`serde` 等）、lib target 同时输出 `cdylib` 与 `staticlib` |
| `src-tauri/build.rs` | 调用 `tauri_build::build()` 完成 Tauri 构建阶段代码生成 |
| `src-tauri/tauri.conf.json` | Tauri 应用配置：产品名称、版本号、标识符、前后端命令联动、窗口与图标、bundle targets |
| `pnpm-lock.yaml` | 前端依赖锁定文件 |
| `Cargo.lock` | Rust 依赖锁定文件 |

## 3. 架构与约定

### 构建流程
1. **开发模式**：`pnpm tauri dev` → Tauri 执行 `beforeDevCommand: pnpm dev` 启动 Vite 开发服务器（监听 `http://localhost:1420`），然后加载 Rust 后端并注入前端页面。`vite.config.ts` 中通过 `TAURI_DEV_HOST` 环境变量配置 HMR 反向连接。
2. **生产构建**：`pnpm tauri build` → Tauri 先执行 `beforeBuildCommand: pnpm build`（即 `vue-tsc --noEmit && vite build`，先做类型检查再产出静态资源到 `../dist`），再用 Rust 编译器编译后端并将 `dist/` 与二进制打包进应用。
3. **纯前端构建**：`pnpm build` 仅产出静态资源到 `dist/`，不生成桌面应用。

### 前后端版本同步约定
- 版本号在三个位置保持一致：`package.json` 的 `version`、`src-tauri/Cargo.toml` 的 `version`、`src-tauri/tauri.conf.json` 的 `version`，均为 `0.1.0`。这构成一个隐式约定——三者需手动同步更新。

### 产物结构
- 前端静态资源输出目录：`dist/`（被 `tauri.conf.json` 的 `frontendDist: "../dist"` 引用）
- Rust crate 名称与 lib 名分离：crate 名为 `hq-kanban-app`，lib 目标命名为 `hq_kanban_app_lib`，并通过 `crate-type = ["lib", "cdylib", "staticlib"]` 同时输出动态库与静态库以适配 Tauri 多平台需求
- bundle 图标覆盖 Windows、macOS、Linux、UWP Store 等多平台格式，位于 `src-tauri/icons/`

### 安全与权限
- `tauri.conf.json` 中 `security.csp: null` 关闭 CSP 限制
- 能力声明位于 `src-tauri/capabilities/default.json`，由 Tauri v2 的能力系统管理

## 4. 约定与约束

- **开发服务器端口固定**：`vite.config.ts` 强制 `server.port = 1420` 且 `strictPort: true`，确保 Tauri 能稳定连接前端 Dev Server。
- **忽略 Rust 源码热重载**：Vite watch 配置显式 `ignored: ["**/src-tauri/**"]`，避免修改 Rust 代码触发前端重新加载。
- **Tauri 构建前置命令**：通过 `tauri.conf.json` 的 `build.beforeDevCommand` 和 `build.beforeBuildCommand` 将前端构建嵌入 Tauri 生命周期，开发者只需运行 `pnpm tauri dev/build` 即可一键完成全栈构建。
- **跨平台打包**：`bundle.targets = "all"` 表示一次构建尝试生成所有支持平台的安装包；图标目录提供多分辨率 PNG、`.icns`、`.ico`、UWP Logo 等格式以满足各平台要求。
- **无外部 CI/Makefile/Dockerfile**：仓库根目录未发现 Makefile、Dockerfile、GitHub Actions/GitLab CI 等自动化流水线文件；构建完全依赖本地 `pnpm` + `cargo` + `tauri` 三件套。
- **依赖锁定**：前端使用 `pnpm-lock.yaml`，Rust 使用 `Cargo.lock`，保证可重复构建。

## 5. 总结

该项目的构建体系围绕 Tauri v2 展开：Tauri 作为编排者，在前端构建（Vite）与原生构建（Cargo）之间建立桥接，通过 `tauri.conf.json` 中的 `beforeDevCommand` / `beforeBuildCommand` / `frontendDist` 实现一体化体验。版本管理是手工同步三个配置文件中的 `version` 字段，未引入自动化的版本提升脚本或 CI 流水线。