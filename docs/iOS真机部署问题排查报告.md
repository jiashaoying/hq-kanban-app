# 大盘行情看板 iOS 真机部署问题排查报告

**项目**：大盘行情看板（Tauri v2 + Vue 3）
**目标设备**：iPhone 17 Pro Max（iOS 27.0）
**日期**：2026-08-31 ~ 2026-09-17

---

## 一、问题总览

应用在 iOS 模拟器和真机上均无法正常运行，经历了七个阶段的问题排查与修复：

| 阶段 | 问题 | 表现 | 状态 |
|---|---|---|---|
| 1 | wry WebView 版本查询崩溃 | 启动即 SIGTRAP 闪退 | ✅ 已解决 |
| 2 | 代码签名配置错误 | EXC_BAD_ACCESS 崩溃 / 签名失败 | ✅ 已解决 |
| 3 | 静态库误打包进 app bundle | dyld4 加载阶段崩溃 | ✅ 已解决 |
| 4 | 生产构建仍连接开发服务器 | 报 `localhost:1420` 连接错误 | ✅ 已解决 |
| 5 | 真机闪退无任何可用日志 | 无从定位，只能靠猜 | ✅ 已解决 |
| 6 | iOS 26+ 未采用 UIScene 生命周期 | 启动即闪退（Runtime Issue） | ✅ 已解决 |
| 7 | tao Scene 配置悬垂指针 | 启动 SIGSEGV（`objc_retain` 野指针） | ✅ 已解决 |


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

## 二之二、阶段 5：真机闪退的定位（日志采集）

> 2026-09-16：执行 `scripts/deploy-ios.sh` 后真机闪退。静态检查已排除签名 / dyld / dev 模式 / wry 补丁 / 资源嵌入等问题，必须靠运行时日志定位。

### 关键机制：iOS 上 Rust panic 就是闪退

`#[cfg_attr(mobile, tauri::mobile_entry_point)]` 展开后包含：

```rust
fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
  match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
    Ok(t) => t,
    Err(err) => {
      eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
      std::process::abort()
    }
  }
}
```

桌面端 panic 只打印堆栈；iOS 端会直接 `abort()`，表现为**无任何 UI 提示的闪退**。

### 日志获取（三步）

1. `lib.rs` 的 panic hook 已改为写入 `$HOME/Documents/tauri_panic.log` —— 真机沙盒内唯一可写、且能被 Xcode「Devices and Simulators → 下载容器」导出的位置。原实现的 `/tmp` 与 `/var/mobile/Containers/Data/Application/` 在真机上均不可写，等于拿不到任何信息。
2. 部署脚本第 7 步改为 `--console` 启动并采集 12 秒日志到 `.deploy-logs/ios-launch-*.log`，自动摘取 `panic` / `unwind` / `abort` / `EXC_` 等关键字。`CONSOLE_SECONDS=30 ./scripts/deploy-ios.sh` 可延长采集时长。
3. 手工复现：`xcrun devicectl device process launch --console --device <UDID> com.huafu.hqkanbanapp`。

### 顺带补上的两个 CLI 缺失步骤

脚本直接调 `xcodebuild`，绕开了 Tauri CLI，因此补了两步：

| 步骤 | 说明 |
|---|---|
| 同步 `dist` → `gen/apple/assets` | CLI 会把 frontendDist 复制到该目录供 Xcode Resources 打包；脚本原先缺失，该目录恒为空 |
| dist 较新时 `touch src-tauri/src/lib.rs` | `generate_context!` 是 proc macro，cargo 监听不到 dist 变化，否则前端改动不会进入 `EmbeddedAssets`（表现为「改了前端但真机还是旧页面」） |

### 已排除项

| 检查项 | 判据 |
|---|---|
| production 模式 | `strings libapp.a \| grep -c "local network permissions"` = 0 |
| 签名 / profile | `codesign -dv` → `TeamIdentifier=QZ5VGE4SZ9`，profile 有效期至 2027-08-28 |
| dyld / 静态库误打包 | app bundle 内无 `libapp.a`，`otool -L` 依赖正常 |
| wry NSBundle 崩溃补丁 | `patches/wry-0.55.1` 生效，iOS 分支走 `NSProcessInfo` |
| 前端资源嵌入 | 4 个 chunk 均在 `EmbeddedAssets` 中 |
| opener 插件 Swift 实现 | `OpenerPlugin.swift.o` 已链接进主二进制 |

---

## 二之三、阶段 6：iOS 26+ UIScene 生命周期闪退（根因）

> 2026-09-17：日志采集就位后复现启动崩溃，控制台关键字落到 UIKit 而非 Rust panic —— `TaoSceneDelegate` 相关 + `EXC_BREAKPOINT (SIGTRAP)`。

### 现象

`get_market_data` 前的 UI 初始化阶段就退出，且 App 进程先出现 `_UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption` 运行时告警，随后 `SIGTRAP`。

### 根因

Apple 在 iOS 26 收紧了 **TN3187**：App 必须采用 UIScene 生命周期，未声明 `UIApplicationSceneManifest` 的 App 会在 `UIApplicationMain` 内被判为 Runtime Issue 并直接中止进程。

### 修复（`Info.plist`）

```xml
<key>UIApplicationSceneManifest</key>
<dict>
  <key>UIApplicationSupportsMultipleScenes</key>
  <true/>
  <key>UISceneConfigurations</key>
  <dict>
    <key>UIWindowSceneSessionRoleApplication</key>
    <array>
      <dict>
        <key>UISceneConfigurationName</key><string>TaoScene</string>
        <key>UISceneDelegateClassName</key><string>TaoSceneDelegate</string>
      </dict>
    </array>
  </dict>
</dict>
```

### 两个容易踩空的点

1. **`UIApplicationSupportsMultipleScenes` 必须为 `true`**：tao 的窗口创建流程（tao `view.rs::create_window_device`）以此开关决定是否调用 `setWindowScene`。置 `false` 时 `UIWindow` 永远挂不上 windowScene，Scene 环境下窗口不显示 —— 不再闪退，但变成**黑屏**。置 `true` 后 tao 会检测到设备实际不支持多场景，转把窗口挂到 main scene，行为正确。
2. **配置名 / delegate 类名要与 tao 对齐**：`TaoScene` / `TaoSceneDelegate`，写错则 Scene 创建失败，症状同样是闪退。

### 部署目标同步提到 iOS 15.0

`IPHONEOS_DEPLOYMENT_TARGET` 由 14.0 → 15.0：14.0 与 Scene 生命周期适配冲突，且与 TN3187 要求的最低基线不符。

### 改动必须落在 `project.yml`

`Info.plist` 与 `pbxproj` 都是 xcodegen 的生成物，直接改会被下一次 `xcodegen generate` 覆盖。已把两处改到真源：

| 改动 | 位置 |
|---|---|
| `deploymentTarget.iOS: 15.0` | `src-tauri/gen/apple/project.yml` → `options` |
| `UIApplicationSceneManifest` | `project.yml` → `targets.hq-kanban-app_iOS.info.properties` |

改完执行 `(cd src-tauri/gen/apple && xcodegen generate)` 并复核生成物：

```bash
grep -n "IPHONEOS_DEPLOYMENT_TARGET" hq-kanban-app.xcodeproj/project.pbxproj   # 应为 15.0
plutil -p hq-kanban-app_iOS/Info.plist | grep -A8 UIApplicationSceneManifest   # 键值齐备、无重复
```

> 顺带发现：`src/kline_cache.rs` 早已加入仓库，但工程未重新生成，pbxproj 内缺其文件引用。本次 `xcodegen generate` 已一并补齐。

---

## 二之四、阶段 7：SIGSEGV 根因 —— tao Scene 配置悬垂指针

> 2026-09-17：Scene Manifest 生效后闪退依旧，但异常类型已从 `EXC_BREAKPOINT` 变为 `EXC_BAD_ACCESS / SIGSEGV`。最终靠**设备崩溃报告**定位到 tao 的所有权 bug。

### 取证：绕开 `--console`，直接拉设备崩溃报告

`devicectl device process launch --console` 对「main 之前就崩溃、且无 stderr 输出」的场景基本无用（只回一行 `App terminated due to signal 11`）。`devicectl` 其实能直接取设备上的系统崩溃日志：

```bash
# 拉取 systemCrashLogs 域（含已符号化的 .ips 崩溃报告）
mkdir -p /tmp/ios-crash
xcrun devicectl device copy from \
  --device <UDID> --domain-type systemCrashLogs \
  --source . --destination /tmp/ios-crash
ls -t /tmp/ios-crash/*.ips

# 解析（第一行是 header，其余是格式化 JSON 正文）
python3 -c "
import json
raw=open('/tmp/ios-crash/大盘行情看板-2026-09-17-105531.ips',encoding='utf-8').read()
i=raw.index('\n{'); d=json.loads(raw[i+1:])
print(d['exception'])
for t in d['threads']:
    if t.get('triggered'):
        [print('  ',f['imageName'],f['symbol']) for f in t['frames'][:10]]
"
```

同类命令：`--domain-type appDataContainer --domain-identifier com.huafu.hqkanbanapp` 可导出 App 沙盒（`Documents/tauri_panic.log` 即从这里取）。

### 崩溃报告给出的结论

```
exception: EXC_BAD_ACCESS (SIGSEGV)  KERN_INVALID_ADDRESS
  0  objc_retain
  1  -[UIApplication _connectUISceneFromFBSScene:transitionContext:]
  2  -[UIApplication workspace:didCreateScene:withTransitionContext:completion:]
  ...
  9  UIApplicationMain
  0  tao::platform_impl::platform::event_loop::EventLoop::run
```

崩溃点在 UIKit 内部 `objc_retain` 一个无效地址 —— 典型「对象已释放却被 retain」。而 `_connectUISceneFromFBSScene:` 恰好是 UIKit 读取 **UISceneConfiguration** 并实例化 Scene delegate 的地方。

### 根因：tao 0.35.3 归还了已释放的 SceneConfiguration

`tao-0.35.3/src/platform_impl/ios/view.rs::configuration_for_connecting_scene_session`（`application:configurationForConnectingSceneSession:options:` 的 IMP）：

```rust
let config = UISceneConfiguration::configurationWithName_sessionRole(...);  // +1
config.setDelegateClass(Some(super::scene::TaoSceneDelegate::class()));
Retained::as_ptr(&config) as _   // ← 只借指针，函数返回时 Retained drop → release → 对象销毁
```

`Retained::as_ptr` 是**借用**，不转移所有权；函数返回后 `config` 被释放，UIKit 拿到悬垂指针，随后 `objc_retain` 即 SIGSEGV。delegate 方法返回值是按 +0 约定，正确写法是交给 autorelease pool：

```rust
Retained::autorelease_return(config) as _
```

### 修复：以 patch 方式改 tao

```toml
# src-tauri/Cargo.toml
[patch.crates-io]
wry = { path = "patches/wry-0.55.1" }
tao = { path = "patches/tao-0.35.3" }   # 新增
```

改完后必须 `cargo build` 重编 tao（path patch 会改写 `Cargo.lock` 的 source）。

### 验证（三项证据）

| 证据 | 结果 |
|---|---|
| 启动后 30s 未被系统终止 | 控制台只出现脚本 `kill` 导致的 `signal 15`，不再有 `signal 11` |
| 手动 launch + 截图 | `devicectl device capture screenshot` 显示行情卡片正常渲染（非黑屏） |
| 再拉崩溃报告 | 11:11 安装新版本后无新增 `.ips` |

---

## 三、经验总结

### 关键教训

1. **xcodegen 项目的同步陷阱**：`project.yml` 是唯一事实来源，但构建实际读取的是 `project.pbxproj`。任何 yml 修改后必须立即 `xcodegen generate` 并 grep 验证 pbxproj 实际内容，否则修改形同虚设。

2. **Tauri 构建模式的可靠判定方法**：不要依赖构建日志判断 dev/production，用 `strings` 检查二进制中的 dev-only 字符串是最直接的证据。

3. **feature 变更必须清缓存**：cargo feature 变更不会自动触发全量重编译验证，排查此类问题时先 `cargo clean` 排除缓存干扰。

4. **真机 ≠ 模拟器**：真机上 `localhost` 指向设备自身，dev 模式必须依赖 Mac 局域网 IP 或直接使用 Release 构建；同时真机必须处理代码签名链（Team → 证书 → Profile → Bundle ID）。

5. **崩溃报告比 console 日志可靠得多**：`devicectl device copy from --domain-type systemCrashLogs` 拉回的 `.ips` 是已符号化的完整堆栈，能把「启动闪退」直接定位到具体函数。反观 `--console`，对 main 之前崩溃、且无 stderr 输出的场景只给一行 `signal 11`。**排查 iOS 闪退的第一步就该是拉崩溃报告**。

6. **objc2 的所有权必须显式区分**：`Retained::as_ptr`（借用，函数返回即失效）与 `Retained::autorelease_return`（+0 返回值交 autorelease pool）语义完全不同。把 `as_ptr` 当返回值用，等于把已释放对象交给系统 —— 症状是随机的 `objc_retain` / `objc_release` 野指针崩溃，且极难从日志反推。

7. **上游依赖的坑要有能力自己补**：`patches/wry-0.55.1`、`patches/tao-0.35.3` 两个补丁都是上游在 iOS 新系统上未适配的问题。升级依赖后需复核补丁是否仍必要（见「遗留注意事项」）。

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

> 上述全流程已封装为 `./scripts/deploy-ios.sh`（含 dist 同步、EmbeddedAssets 强制重建、production 校验、控制台日志采集）。

### 遗留注意事项

- wry patch（`patches/wry-0.55.1`）是针对 0.55.1 的临时方案，升级 wry 后需确认官方是否已修复 iOS 26+ 的 NSBundle 崩溃问题
- tao patch（`patches/tao-0.35.3`）修复 `configuration_for_connecting_scene_session` 的悬垂指针，升级 tao 后需确认上游是否已改用 `autorelease_return`
- tao 中仍存在用裸指针存 `UIWindow` 的写法（`window.rs` 的 `From<Retained<UIWindow>> for WindowId`），当前未观察到问题，后续若出现窗口相关的悬垂访问可从这里查
- `tauri.conf.json`、`project.yml`、`project.pbxproj` 三处签名/Bundle ID 配置需保持一致，xcodegen 重新生成后以 yml 为准
- IPA 导出（`pnpm tauri ios build --export-method debugging`）可用于分发测试，但项目使用自定义 preBuildScript 时需确保 feature 标志已同步
