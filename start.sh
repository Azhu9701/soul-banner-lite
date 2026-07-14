#!/bin/bash
# Snake Skin — 一键启动脚本
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"

# Auto-detect LAN IP for mobile access
LAN_IP=$(ipconfig getifaddr en0 2>/dev/null || echo "")

echo "========================================"
echo "  Snake Skin"
echo "  模拟仲裁庭 · 劳动争议"
echo "========================================"

cleanup() {
  echo ""
  echo "正在关闭服务..."
  kill $API_PID 2>/dev/null
  kill $FRONT_PID 2>/dev/null
  exit 0
}
trap cleanup SIGINT SIGTERM

# Start API
echo "[1/2] 启动 API 服务 (0.0.0.0:3096)..."
cd "$DIR"
cargo run -p api 2>&1 | sed 's/^/[API] /' &
API_PID=$!
sleep 2

# Wait for API
for i in $(seq 1 20); do
  if curl -s http://127.0.0.1:3096/api/v1/health > /dev/null 2>&1; then
    echo "[1/2] API 就绪 ✓"
    break
  fi
  sleep 1
done

# Start Frontend (set API URL for mobile access)
echo "[2/2] 启动前端 (http://localhost:3000)..."
cd "$DIR/nextjs"
NEXT_PUBLIC_API_URL="http://${LAN_IP:-127.0.0.1}:3096/api/v1" pnpm dev 2>&1 | sed 's/^/[WEB] /' &
FRONT_PID=$!
sleep 3

echo ""
echo "========================================"
echo "  电脑访问: http://localhost:3000"
if [ -n "$LAN_IP" ]; then
  echo "  手机访问: http://${LAN_IP}:3000"
fi
echo "  API:      http://0.0.0.0:3096"
echo "  Ctrl+C    关闭所有服务"
echo "========================================"

# Open browser
sleep 1
open http://localhost:3000 2>/dev/null || true

wait
