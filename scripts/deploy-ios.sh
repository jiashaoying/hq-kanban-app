#!/bin/bash
# =============================================================
# 大盘行情看板 — iOS 真机 Release 一键部署脚本
#
# 用法:
#   ./scripts/deploy-ios.sh                 # 使用默认设备
#   ./scripts/deploy-ios.sh <设备UDID>      # 指定设备 UDID
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

export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

cd "$PROJECT_ROOT"

step() { echo -e "\n========== $1 =========="; }

# ---- 1. 前端生产构建 ----
step "1/5 前端生产构建 (pnpm build)"
pnpm build

# ---- 2. 构建配置检查（production 关键开关） ----
step "2/5 检查 pbxproj 是否含 custom-protocol feature"
if ! grep -q "features tauri/custom-protocol" "$XCODEPROJ/project.pbxproj"; then
  echo "错误：pbxproj 中缺少 --features tauri/custom-protocol！"
  echo "如果修改过 project.yml，请先运行: (cd src-tauri/gen/apple && xcodegen generate)"
  exit 1
fi
echo "OK：production 编译开关就位"

# ---- 3. Xcode Release 构建到真机 ----
step "3/5 xcodebuild Release 构建 (设备: $DEVICE_ID)"
xcodebuild -project "$XCODEPROJ" \
  -scheme "$SCHEME" \
  -configuration release \
  -destination "id=$DEVICE_ID" \
  -allowProvisioningUpdates build

# ---- 4. Production 模式验证 ----
step "4/5 验证 production 编译生效"
DEV_COUNT=$(strings "$LIBAPP_A" 2>/dev/null | grep -c "local network permissions" || true)
if [ "$DEV_COUNT" != "0" ]; then
  echo "错误：检测到 dev 代理代码（localhost 错误将复发）！执行以下命令后重试:"
  echo "  cargo clean --target aarch64-apple-ios --release"
  exit 1
fi
echo "OK：无 dev 代理代码，production 模式生效"

# ---- 5. 安装并启动 ----
step "5/5 安装并启动到真机"
xcrun devicectl device install app --device "$DEVICE_ID" "$APP_PATH"
xcrun devicectl device process launch --device "$DEVICE_ID" "$BUNDLE_ID"

echo -e "\n✅ 部署完成，应用已在真机启动"
