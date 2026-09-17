#!/bin/bash
# =============================================================
# 大盘行情看板 — iOS 真机 Release 一键部署脚本
#
# 用法:
#   ./scripts/deploy-ios.sh                 # 使用默认设备
#   ./scripts/deploy-ios.sh <设备UDID>      # 指定设备 UDID
#   CONSOLE_SECONDS=30 ./scripts/deploy-ios.sh   # 延长控制台日志采集时长
#
# 前提:
#   - 已安装 xcodegen / rustup 工具链（iOS targets）
#   - 真机已通过 USB 连接并信任本机
#   - project.yml 有改动时需先执行 xcodegen generate（本脚本不自动执行，防止覆盖 Xcode 中的手动配置）
# =============================================================
set -euo pipefail

# ---- 配置 ----
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEVICE_ID="${1:-00008150-000A31163407801C}"   # iPhone 17 Pro Max
BUNDLE_ID="com.huafu.hqkanbanapp"
DERIVED_DIR="hq-kanban-app-egeuxspoucajqvcwmyxrfxxssacy"
XCODEPROJ="src-tauri/gen/apple/hq-kanban-app.xcodeproj"
SCHEME="hq-kanban-app_iOS"
APP_NAME="大盘行情看板"
APP_PATH="$HOME/Library/Developer/Xcode/DerivedData/$DERIVED_DIR/Build/Products/release-iphoneos/$APP_NAME.app"
LIBAPP_A="src-tauri/gen/apple/Externals/arm64/release/libapp.a"
IOS_ASSETS="src-tauri/gen/apple/assets"
CARGO_LIB_A="src-tauri/target/aarch64-apple-ios/release/libhq_kanban_app_lib.a"
CONSOLE_SECONDS="${CONSOLE_SECONDS:-12}"

export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

cd "$PROJECT_ROOT"

step() { echo -e "\n========== $1 =========="; }

# ---- 1. 前端生产构建 ----
step "1/7 前端生产构建 (pnpm build)"
pnpm build

# ---- 2. 同步前端产物到 Xcode assets 资源目录 ----
# Tauri CLI 的 iOS 流程会把 frontendDist 复制到 gen/apple/assets，再由 Xcode
# 作为 Resources 打进 bundle。本脚本直接调 xcodebuild 绕开了 CLI，必须手动补这一步。
step "2/7 同步 dist 到 $IOS_ASSETS"
rm -rf "$IOS_ASSETS"
mkdir -p "$IOS_ASSETS"
cp -R dist/. "$IOS_ASSETS/"
# 目录需常驻：Xcode 的 Resources build phase 引用了它，缺失会导致构建报资源找不到
touch "$IOS_ASSETS/.gitkeep"
echo "OK：已同步 $(find "$IOS_ASSETS" -type f ! -name .gitkeep | wc -l | tr -d ' ') 个文件"

# ---- 3. 确保前端变更能触发 Rust 重编译 ----
# generate_context! 是 proc macro，前端资源以 EmbeddedAssets 形式在编译期嵌入。
# cargo 监听不到 dist 的变化，Rust 源码未改动时不会重跑 proc macro，
# 导致「前端改了但真机上还是旧页面」。用 touch 强制其重跑。
step "3/7 检查 EmbeddedAssets 新鲜度"
if [ ! -f "$CARGO_LIB_A" ] || [ dist/index.html -nt "$CARGO_LIB_A" ]; then
  echo "dist 比 Rust 产物新 → touch lib.rs 强制重建 EmbeddedAssets"
  touch src-tauri/src/lib.rs
else
  echo "OK：Rust 产物已包含最新前端"
fi

# ---- 4. 构建配置检查（production 关键开关） ----
step "4/7 检查 pbxproj 是否含 custom-protocol feature"
if ! grep -q "features tauri/custom-protocol" "$XCODEPROJ/project.pbxproj"; then
  echo "错误：pbxproj 中缺少 --features tauri/custom-protocol！"
  echo "如果修改过 project.yml，请先运行: (cd src-tauri/gen/apple && xcodegen generate)"
  exit 1
fi
echo "OK：production 编译开关就位"

# ---- 5. Xcode Release 构建到真机 ----
step "5/7 xcodebuild Release 构建 (设备: $DEVICE_ID)"
xcodebuild -project "$XCODEPROJ" \
  -scheme "$SCHEME" \
  -configuration release \
  -destination "id=$DEVICE_ID" \
  -allowProvisioningUpdates build

# ---- 6. Production 模式验证 ----
step "6/7 验证 production 编译生效"
DEV_COUNT=$(strings "$LIBAPP_A" 2>/dev/null | grep -c "local network permissions" || true)
if [ "$DEV_COUNT" != "0" ]; then
  echo "错误：检测到 dev 代理代码（localhost 错误将复发）！执行以下命令后重试:"
  echo "  cargo clean --target aarch64-apple-ios --release"
  exit 1
fi
echo "OK：无 dev 代理代码，production 模式生效"

# ---- 7. 安装并启动（同时采集控制台日志） ----
# iOS 上 Rust panic 会被 mobile_entry_point 展开出的 stop_unwind 转成 abort()，
# 表现为纯闪退、无任何 UI 提示。只有 --console 能抓到 panic / 崩溃瞬间的输出。
step "7/7 安装并启动到真机（采集 ${CONSOLE_SECONDS}s 控制台日志）"
xcrun devicectl device install app --device "$DEVICE_ID" "$APP_PATH"

LOG_DIR="$PROJECT_ROOT/.deploy-logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/ios-launch-$(date +%Y%m%d-%H%M%S).log"
: >"$LOG_FILE"

xcrun devicectl device process launch --console --device "$DEVICE_ID" "$BUNDLE_ID" >"$LOG_FILE" 2>&1 &
CONSOLE_PID=$!
sleep "$CONSOLE_SECONDS"
kill "$CONSOLE_PID" 2>/dev/null || true
wait "$CONSOLE_PID" 2>/dev/null || true

echo -e "\n控制台日志: $LOG_FILE"
echo "---------- 异常线索（自动摘取） ----------"
grep -aiE "panic|unwind|abort|SIGABRT|SIGSEGV|SIGTRAP|EXC_|terminat|crash|not found|denied|error" "$LOG_FILE" | head -40 || echo "（无匹配，启动阶段未输出异常）"
echo "----------------------------------------"

echo -e "\n✅ 部署完成。若应用闪退，请查看上面的线索，或执行:"
echo "   cat $LOG_FILE"
