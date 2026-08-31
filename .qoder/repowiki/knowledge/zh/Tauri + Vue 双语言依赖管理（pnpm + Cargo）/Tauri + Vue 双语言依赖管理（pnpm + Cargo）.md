---
kind: dependency_management
name: Tauri + Vue 双语言依赖管理（pnpm + Cargo）
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - pnpm-lock.yaml
    - src-tauri/Cargo.toml
    - src-tauri/Cargo.lock
---

## 1. 使用的系统与工具

本项目采用**双语言、双包管理器**的依赖管理模式：
- **前端（Vue 3 + Vite + TypeScript）**：使用 **pnpm v11.24.0** 作为包管理器，通过 `package.json` 声明依赖，并通过仓库根目录的 `pnpm-lock.yaml`（lockfileVersion '9.0'）锁定所有依赖的确切版本与完整性校验哈希。
- **后端（Rust / Tauri v2）**：使用 **Cargo** 作为包管理器，通过 `src-tauri/Cargo.toml` 声明 Rust crate 依赖，并通过 `src-tauri/Cargo.lock` 锁定编译产物。

项目未使用 npm/yarn，也未在仓库中 vendoring 第三方源码（无 `vendor/`、`third_party/` 等目录），完全依赖远程 registry 解析。

## 2. 关键文件

| 文件 | 作用 |
|---|---|
| `package.json` | 声明前端运行时依赖（`vue`、`@tauri-apps/api`）与开发依赖（`vite`、`typescript`、`@vitejs/plugin-vue`、`tailwindcss`、`@tauri-apps/cli`、`vue-tsc`） |
| `pnpm-lock.yaml` | pnpm 锁文件，锁定 pnpm 自身版本（11.24.0）及全部依赖树，包含平台特定二进制（如 `@pnpm/linux-x64`、`@reflink/reflink-darwin-arm64` 等） |
| `src-tauri/Cargo.toml` | Rust crate 清单，声明 `tauri`、`tauri-plugin-opener`、`serde`、`serde_json`、`reqwest`、`encoding_rs`、`tokio` 等依赖 |
| `src-tauri/Cargo.lock` | Cargo 锁文件，锁定 Rust 依赖树 |
| `src-tauri/tauri.conf.json` | Tauri 应用配置（非依赖声明文件，但决定打包时嵌入的前端产物） |

## 3. 架构与约定

### 前端依赖策略
- 生产依赖仅两个：`vue`（`^3.5.41`）和 `@tauri-apps/api`（`^2.11.1`），保持最小化。
- 构建/类型检查/样式相关工具统一放在 `devDependencies`：Vite 8、TypeScript 5、Vue 编译器插件、Tailwind CSS 4、`vue-tsc`、`@tauri-apps/cli`。
- 版本号普遍使用 `^` 前缀（caret range），允许小版本/补丁自动升级；但实际安装版本由 `pnpm-lock.yaml` 精确固定。
- 通过 `pnpm` 的 `packageManagerDependencies` 字段将 pnpm 自身版本（11.24.0）也锁定，确保团队与 CI 环境一致。

### Rust 依赖策略
- 使用语义化版本范围：`tauri = { version = "2", ... }`、`serde = { version = "1", features = ["derive"] }`、`reqwest = { version = "0.12", features = ["blocking"] }`、`tokio = { version = "1", features = ["full"] }` 等，均只指定主版本或次版本范围。
- 通过 `features` 按需启用能力（如 `serde` 的 `derive`、`reqwest` 的 `blocking`、`tokio` 的 `full`），避免引入不必要的功能。
- `build-dependencies` 与 `dependencies` 分离，`tauri-build` 仅用于构建阶段。

### 跨语言协作
- 前端通过 `@tauri-apps/api` 调用 Rust 暴露的命令（IPC），Rust 侧通过 `tauri` crate 提供命令接口，形成前后端依赖契约。
- 前端构建产物由 `vite build` 生成并嵌入到 Tauri 应用中，因此前端依赖的版本直接影响最终桌面应用的体积与行为。

## 4. 约定与约束

- **锁文件必须提交**：`pnpm-lock.yaml` 与 `src-tauri/Cargo.lock` 均在仓库中，保证可重现构建。新增依赖后需更新对应 lock 文件。
- **不 vendoring 源码**：所有第三方代码通过 pnpm registry 与 crates.io 拉取，不在仓库内维护副本。
- **私有注册表/镜像**：未发现 `.npmrc`、`.pnpmrc`、`Cargo.toml` 中的 `[source]` 或 `config.toml` 中的自定义 registry 配置，默认使用官方源。
- **版本升级策略**：依赖声明使用 caret 范围（`^`），由包管理器根据 lock 文件确定具体版本；如需强制升级，应修改 `package.json`/`Cargo.toml` 后重新生成 lock 文件。
- **脚本入口**：`package.json` 的 `scripts` 定义了 `dev`、`build`、`preview`、`tauri` 四个命令，其中 `tauri` 直接转发给 `@tauri-apps/cli`，是构建/发布桌面应用的统一入口。
- **平台原生依赖**：pnpm 锁文件中包含大量平台特定的可选依赖（如 `@pnpm/linux-*`、`@reflink/reflink-*-arm64`），这些会在安装时按当前平台自动选择，无需手动配置。

## 5. 总结

该项目以 pnpm + Cargo 双管齐下管理依赖，前端追求最小运行时依赖、后端通过 features 精细控制编译产物，两者都通过 lock 文件保证构建可重现。没有发现私有 registry、vendoring 或 monorepo workspace 配置，属于典型的单仓双语言桌面应用依赖管理模式。