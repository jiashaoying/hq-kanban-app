---
kind: error_handling
name: Tauri 桌面应用错误处理：Rust Result 与前端 try/catch 的轻量级协作
category: error_handling
scope:
    - '**'
source_files:
    - src-tauri/src/lib.rs
    - src-tauri/src/market.rs
    - src-tauri/src/main.rs
    - src/composables/useMarketData.ts
---

## 1. 整体方案

该仓库是一个基于 Tauri v2 + Vue 3 的跨平台桌面应用，错误处理采用前后端各自语言惯用的轻量方式：
- Rust 后端使用 `Result<T, E>` 配合 `?` 操作符进行错误传播，并通过 Tauri 命令返回 `String` 作为错误信息。
- 前端通过 `@tauri-apps/api/core` 的 `invoke` 调用 Rust 命令，使用 `try/catch` 捕获异常并降级为 UI 状态（loading 置 false、控制台输出错误）。

没有自定义错误类型、没有全局错误中间件、没有 panic/recover 策略，属于最小化实现。

## 2. 关键文件与位置

- `src-tauri/src/lib.rs`：定义 Tauri 命令入口。`fetch_indices` 命令返回 `Result<Vec<IndexData>, String>`，将底层错误通过 `.map_err(|e| e.to_string())` 转为字符串；`run()` 中用 `.expect("error while running tauri application")` 在启动失败时直接 panic。
- `src-tauri/src/market.rs`：核心数据抓取逻辑。`fetch_all_indices()` 返回 `Result<Vec<IndexData>, Box<dyn std::error::Error>>`，使用 `reqwest::get(...).await?` 和 `response.bytes().await?` 通过 `?` 传播网络/解析错误；字段解析使用 `parse_f64` 辅助函数，内部对空串或非法数字使用 `unwrap_or(0.0)` 做容错而非抛错。
- `src/composables/useMarketData.ts`：前端轮询模块。`refresh()` 包裹 `try/catch`，成功时更新 `indices` 和 `lastUpdateTime`，失败时仅 `console.error('Failed to fetch indices:', e)`，并在 `finally` 中将 `loading` 重置为 false。
- `src-tauri/src/main.rs`：仅调用库的 `run()`，无额外错误处理。

## 3. 架构与约定

### Rust 侧
- **命令返回值约定**：所有 Tauri 命令统一以 `Result<T, String>` 形式暴露给前端（见 `fetch_indices`），错误被序列化为字符串后由 Tauri 框架传递给前端 `invoke` 的 reject。
- **错误传播链**：业务层 `market.rs` 返回 `Box<dyn std::error::Error>` → 命令层 `lib.rs` 通过 `.map_err(e.to_string())` 转换为 `String` → Tauri 自动序列化到前端。
- **网络 I/O 错误**：全部通过 `?` 短路返回，不尝试分类或包装。
- **数据解析容错**：`parse_f64` 对空串/非法数值返回默认值 `0.0`，避免整条记录因单个字段解析失败而丢弃。
- **启动期致命错误**：`run()` 末尾 `.expect(...)` 在初始化失败时直接 panic，由操作系统终止进程。

### 前端侧
- **调用层统一 try/catch**：`useMarketData.ts` 是唯一调用 `invoke('fetch_indices')` 的地方，集中捕获错误并降级。
- **UI 状态驱动的错误恢复**：无论成功或失败，`loading` 都会通过 `finally` 复位，保证 UI 不会卡在 loading 状态。
- **无用户可见的错误提示**：当前实现仅 `console.error`，未弹出 toast 或对话框；错误对用户不可见，仅开发者可观测。

## 4. 约定与约束

- **禁止 panic 于业务路径**：业务逻辑（如 `market.rs`）不使用 `panic!`，而是返回 `Result`；仅在应用启动阶段允许 `.expect()` 式 panic。
- **数值解析必须容错**：通过 `parse_f64` 统一处理空/非法数字，禁止直接使用 `unwrap()` 解析可能失败的字段。
- **Tauri 命令错误必须为 `String`**：命令签名固定为 `Result<T, String>`，确保前端 `invoke` 能收到可读的错误消息。
- **前端错误不中断轮询**：`catch` 块不 rethrow，也不停止定时器，下一次 `POLL_INTERVAL` 到期仍会重试。
- **无全局错误处理器**：未发现全局异常拦截器、日志上报或错误聚合机制；每个调用点自行决定如何处理错误。

## 5. 缺失/待改进之处（基于代码现状的描述性观察）

- 前端未区分网络错误、解析错误等语义，无法向用户展示差异化提示。
- 未实现重试退避、超时控制、取消轮询时的错误清理。
- 未对 Tauri 命令注册失败等更早期错误进行处理。
- 未使用结构化日志（如 `tracing`/`log`），错误仅落盘到浏览器控制台。