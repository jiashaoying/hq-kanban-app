---
kind: external_dependency
name: Tauri 2 桌面应用框架
slug: tauri
category: external_dependency
category_hints:
    - framework_behavior
scope:
    - '**'
---

### Tauri 2（基于 Rust + WebView）
- 角色：跨平台桌面应用容器，将 Vue 3 前端打包进 .app/.dmg/.msi/.exe，运行在系统原生 WebView（macOS WebKit、Windows WebView2）中
- 集成点：`src-tauri/`（Rust 后端）、`tauri.conf.json`（窗口尺寸、标题、CSP、图标、bundle targets）、`capabilities/default.json`（窗口拖动/最小化/关闭权限）
- 注意：本应用使用无边框窗口（`decorations: false`），自定义标题栏承载拖动区域；CSP 设为 null 以允许本地资源加载