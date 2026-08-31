# 大盘行情看板 iOS 真机部署问题排查报告

**项目**：大盘行情看板（Tauri v2 + Vue 3）
**目标设备**：iPhone 17 Pro Max（iOS 27.0）
**日期**：2026-08-31

---

## 一、问题总览

应用在 iOS 模拟器和真机上均无法正常运行，经历了四个阶段的问题排查与修复：

| 阶段 | 问题 | 表现 | 状态 |
|---|---|---|---|
| 1 | wry WebView 版本查询崩溃 | 启动即 SIGTRAP 闪退 | ✅ 已解决 |
| 2 | 代码签名配置错误 | EXC_BAD_ACCESS 崩溃 / 签名失败 | ✅ 已解决 |
| 3 | 静态库误打包进 app bundle | dyld4 加载阶段崩溃 | ✅ 已解决 |
| 4 | 生产构建仍连接开发服务器 | 报 `localhost:1420` 连接错误 | ✅ 已解决 |

---

## 二、问题详细分析

### 问题 1：wry `platform_webview_version()` 崩溃

**现象**：应用启动约 1 秒后闪退，Xcode 调试器显示 `Thread 1: EXC_BREAKPOINT`，堆栈定位到 `wry::wkwebview::platform_webview_version`。

**根本原因**：wry 0.55.1 源码中该函数调用 `NSBundle::bundleWithIdentifier("com.apple.WebKit")` 获取 WebKit 版本。该调用在 iOS 26.5+ 系统（模拟器和真机均受影响）上会触发 CoreFoundation 内部 `CFRelease` 崩溃，属于 Apple 系统 API 的已知问题（参考 Apple Developer Forums thread 809860）。

**修复方案**：
1. 将 wry 0.55.1 源码复制到 `src-tauri/patches/wry-0.55.1/`
2. 修改 `platform_webview_version()`：iOS 平台改用 `NSProcessInfo::operatingSystemVersionString()` 获取版本信息，完全绕开崩溃的 API；macOS 平台保留原逻辑
3. 在 `src-tauri/Cargo.toml` 中添加 `[patch.crates-io]` 覆盖：

```toml
[patch.crates-io]
wry = { path = "patches/wry-0.55.1" }
```

**验证**：模拟器和真机均不再闪退。

---

### 问题 2：代码签名配置错误

**现象**：真机启动时 `EXC_BAD_ACCESS (code=1, address=0x0)`，崩溃发生在 dyld4 加载阶段（应用代码运行之前）；后续还出现签名失败报错。

**根本原因**：三个独立的签名问题叠加：
1. `tauri.conf.json` 中 `developmentTeam` 配置的团队 ID（`4PX4843PPS`）与本机证书不匹配
2. 原 Bundle ID `com.jiashaoying.hqkanbanapp` 已被其他开发者账号注册，无法在当前团队下使用
3. 签名证书与所选团队不一致（证书属于组织账号，团队却选了个人账号）

**修复方案**：
1. 通过 `security find-identity -v -p codesigning` 确认本机可用签名身份
2. Bundle ID 更换为 `com.huafu.hqkanbanapp`（避免全局注册冲突）
3. `tauri.conf.json` 与 `project.yml` 中统一设置 `developmentTeam: QZ5VGE4SZ9`（HUAFU SECURITIES CO.,LTD）
4. 在 `project.yml` 的 settingGroups 中固化 `DEVELOPMENT_TEAM`，避免 xcodegen 重新生成后丢失

**验证**：`codesign -dv` 显示 `TeamIdentifier=QZ5VGE4SZ9`，应用可正常安装。

---

### 问题 3：静态库误打包进 app bundle

**现象**：签名修复后真机仍出现 `EXC_BAD_ACCESS (address=0x0)` dyld4 阶段崩溃。

**根本原因**：`project.yml` 将 `Externals` 目录作为源码组（`- path: Externals`）加入 Xcode 项目，导致 414MB 的 debug 静态库 `libapp.a` 被当作资源文件复制到 app bundle 根目录，干扰 dyld 加载。

**修复方案**：
1. 删除 `project.yml` sources 中的 `- path: Externals` 配置，静态库仅通过 `dependencies: - framework: libapp.a` 引用
2. 执行 `xcodegen generate` 重新生成项目
3. 验证 app bundle 中不再包含 `libapp.a`

**验证**：dyld4 崩溃消失，应用能启动到 WebView 加载阶段。

---

### 问题 4：生产构建仍连接开发服务器（localhost:1420）

**现象**：应用能启动但显示错误：`Failed to request http://localhost:1420/ ... did you grant local network permissions?`

**根本原因（多层叠加，最易被忽视）**：

1. **Tauri 的 dev/production 判定机制**：Tauri 依据 tauri crate 的 `custom-protocol` cargo feature 区分构建模式。未启用该 feature 时，Tauri 编译 `#[cfg(dev)]` 代码路径（`proxy_dev_request`），运行时 WebView 加载 `tauri.conf.json` 中的 `devUrl`（localhost:1420）而非打包的 `frontendDist`。

2. **本项目使用 xcodegen 自定义构建**：Rust 编译由 `project.yml` 的 preBuildScripts 执行，而非 Tauri CLI 直接控制。即使使用 `pnpm tauri ios build`，自定义脚本也会覆盖其构建行为。

3. **核心失误——project.yml 与 pbxproj 失同步**：构建脚本中添加 `--features tauri/custom-protocol` 后，**忘记重新运行 `xcodegen generate`**，导致 `project.pbxproj` 中的 shellScript 仍是旧版本，feature 从未生效。Rust 代码一直以 dev 模式编译。

**修复方案**：

`project.yml` 中 preBuildScripts 最终形态：

```yaml
preBuildScripts:
  - script: |
      set -e
      export PATH="$HOME/.cargo/bin:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/bin:$PATH"
      cd "${SRCROOT}/../../../"
      if [ "${PLATFORM_DISPLAY_NAME}" = "iOS Simulator" ]; then
        TARGET="aarch64-apple-ios-sim"
      else
        TARGET="aarch64-apple-ios"
      fi
      if [ "${CONFIGURATION}" = "release" ]; then
        CARGO_FLAGS="--release --features tauri/custom-protocol"
      fi
      cargo build --target "$TARGET" --lib --manifest-path src-tauri/Cargo.toml $CARGO_FLAGS
      mkdir -p "${SRCROOT}/Externals/arm64/${CONFIGURATION}"
      cp "src-tauri/target/${TARGET}/${CONFIGURATION}/libhq_kanban_app_lib.a" "${SRCROOT}/Externals/arm64/${CONFIGURATION}/libapp.a"
    name: Build Rust Code
    basedOnDependencyAnalysis: false
```

执行步骤：
1. 修改 `project.yml` 后**必须**运行 `xcodegen generate`
2. `grep CARGO_FLAGS project.pbxproj` 验证脚本内容已同步
3. `cargo clean --target aarch64-apple-ios --release` 清理缓存（feature 变更必须重新编译）
4. xcodebuild release 构建、安装、启动

**验证方法（可靠判据）**：

```bash
strings libapp.a | grep -c "local network permissions"
```

该字符串仅存在于 Tauri dev 模式编译的错误分支（`tauri-2.11.5/src/protocol/tauri.rs` 的 `proxy_dev_request`）。修复前计数为 1，修复后为 **0**，确凿证明 production 模式已生效。

**验证结果**：真机应用正常显示行情数据（上证指数、深证成指、港股、美股等），轮询正常工作。

---

## 三、经验总结

### 关键教训

1. **xcodegen 项目的同步陷阱**：`project.yml` 是唯一事实来源，但构建实际读取的是 `project.pbxproj`。任何 yml 修改后必须立即 `xcodegen generate` 并 grep 验证 pbxproj 实际内容，否则修改形同虚设。

2. **Tauri 构建模式的可靠判定方法**：不要依赖构建日志判断 dev/production，用 `strings` 检查二进制中的 dev-only 字符串是最直接的证据。

3. **feature 变更必须清缓存**：cargo feature 变更不会自动触发全量重编译验证，排查此类问题时先 `cargo clean` 排除缓存干扰。

4. **真机 ≠ 模拟器**：真机上 `localhost` 指向设备自身，dev 模式必须依赖 Mac 局域网 IP 或直接使用 Release 构建；同时真机必须处理代码签名链（Team → 证书 → Profile → Bundle ID）。

### 最终构建命令（真机 Release 部署）

```bash
cd src-tauri/gen/apple
xcodegen generate   # project.yml 有改动时
xcodebuild -project hq-kanban-app.xcodeproj \
  -scheme hq-kanban-app_iOS \
  -configuration release \
  -destination 'id=<设备UDID>' \
  -allowProvisioningUpdates build
xcrun devicectl device install app --device <设备UDID> <DerivedData中的.app路径>
xcrun devicectl device process launch --device <设备UDID> com.huafu.hqkanbanapp
```

### 遗留注意事项

- wry patch（`patches/wry-0.55.1`）是针对 0.55.1 的临时方案，升级 wry 后需确认官方是否已修复 iOS 26+ 的 NSBundle 崩溃问题
- `tauri.conf.json`、`project.yml`、`project.pbxproj` 三处签名/Bundle ID 配置需保持一致，xcodegen 重新生成后以 yml 为准
- IPA 导出（`pnpm tauri ios build --export-method debugging`）可用于分发测试，但项目使用自定义 preBuildScript 时需确保 feature 标志已同步
