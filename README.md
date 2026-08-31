# 大盘行情看板

基于 Tauri v2 的跨平台行情看板，实时展示 A 股、港股、美股主要指数行情，支持 macOS 桌面与 iOS（模拟器 / 真机）。

## 功能特性

- **实时指数行情**：上证指数、深证成指、创业板指、恒生指数、道琼斯、纳斯达克等，按市场分组卡片展示
- **自动轮询刷新**：每 3 秒从腾讯财经接口拉取最新数据
- **涨跌可视化**：红涨绿跌、涨跌幅与成交额格式化显示
- **跨平台**：一套代码运行于 macOS / iOS

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 + TypeScript + Vite 8 + Tailwind CSS 4 |
| 后端 | Rust + Tauri v2 + reqwest + encoding_rs（GBK 解码） |
| 数据源 | 腾讯财经实时行情接口（qt.gtimg.cn） |

## 项目结构

```
├── src/                  # Vue 前端（组件 / composable / 类型）
├── src-tauri/            # Rust 后端
│   ├── src/market.rs     #   行情抓取与解析
│   ├── tauri.conf.json   #   Tauri 配置
│   └── patches/wry-0.55.1/  # wry 补丁（修复 iOS 26+ NSBundle 崩溃）
├── docs/                 # 文档（部署报告 / 命令手册 / 新手指南）
└── dist/                 # 前端构建产物（构建生成）
```

## 快速开始

```bash
# 安装依赖
pnpm install
```

## macOS 运行

```bash
# 开发调试（热更新）
pnpm tauri dev

# 发布构建（类型检查 → Vite 构建 → Rust release 编译 → 打包）
pnpm tauri build

# 运行构建产物
open src-tauri/target/release/bundle/macos/大盘行情看板.app

# 打开 DMG 所在目录（双击 dmg 拖入应用程序完成安装）
open src-tauri/target/release/bundle/dmg/
```

> 产物位置：
> - `.app`：`src-tauri/target/release/bundle/macos/大盘行情看板.app`，双击运行或拖入「应用程序」文件夹安装
> - `.dmg`：`src-tauri/target/release/bundle/dmg/大盘行情看板_0.1.0_aarch64.dmg`，可分发给其他 macOS 用户
>
> 桌面窗口使用自定义标题栏（`decorations: false`），按住顶部标题栏可拖动窗口。

## iOS 运行

```bash
# 模拟器（开发调试）
pnpm tauri ios dev

# 真机（发布模式，详见 docs/构建调试命令手册.md）
pnpm build
cd src-tauri/gen/apple && xcodegen generate
xcodebuild -project hq-kanban-app.xcodeproj \
  -scheme hq-kanban-app_iOS -configuration release \
  -destination 'id=<设备UDID>' -allowProvisioningUpdates build
xcrun devicectl device install app --device <设备UDID> <DerivedData中的.app路径>
```

> 真机必须使用 **release** 配置：debug 模式会连接开发服务器 `localhost:1420`，真机上无法访问。release 构建需启用 `tauri/custom-protocol` feature（已在 `project.yml` 的 preBuildScripts 中配置）。

## 文档

- [构建调试命令手册](docs/构建调试命令手册.md) — 全平台构建、验证与排查命令
- [新手入门指南](docs/新手入门指南.md) — 面向新手的开发调试与真机部署教程
- [iOS 真机部署问题排查报告](docs/iOS真机部署问题排查报告.md) — 崩溃与签名问题的完整排查记录

## 许可证

[MIT](LICENSE) © 2026 贾少英

## 注意事项

- Rust 工具链使用 rustup 版本（Homebrew cargo 缺少 iOS target）
- 修改 `src-tauri/gen/apple/project.yml` 后必须执行 `xcodegen generate`
- 签名配置（Bundle ID `com.huafu.hqkanbanapp` / Team `QZ5VGE4SZ9`）在 `tauri.conf.json` 与 `project.yml` 中需保持一致
