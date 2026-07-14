#!/bin/bash
set -e

# Snake Skin — 一键安装脚本
# 支持本地算力运行（LM Studio）和云端 API 混合模式

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()   { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()  { echo -e "${RED}[ERROR]${NC} $1"; }

print_banner() {
    echo ""
    echo "============================================================"
    echo "  Snake Skin"
    echo "  AI 模拟仲裁庭"
    echo "============================================================"
    echo ""
}

check_cmd() {
    if command -v "$1" &> /dev/null; then
        log_ok "$1 已安装"
        return 0
    else
        return 1
    fi
}

check_deps() {
    log_info "检查前置依赖..."
    local missing=()

    if ! check_cmd "rustc"; then
        missing+=("Rust")
    else
        local rust_version
        rust_version=$(rustc --version | awk '{print $2}')
        log_info "Rust 版本: $rust_version"
    fi

    if ! check_cmd "node"; then
        missing+=("Node.js")
    else
        local node_version
        node_version=$(node --version | sed 's/v//')
        log_info "Node.js 版本: $node_version"
    fi

    if ! check_cmd "pnpm"; then
        missing+=("pnpm")
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        log_err "缺少以下依赖: ${missing[*]}"
        echo ""
        echo "安装方法:"
        echo ""
        echo "  Rust:    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  Node.js: https://nodejs.org/ (推荐 18+)"
        echo "  pnpm:    npm install -g pnpm"
        echo ""
        exit 1
    fi

    log_ok "所有依赖已就绪"
}

setup_data_dir() {
    log_info "初始化数据目录..."
    mkdir -p "$SCRIPT_DIR/data"
    mkdir -p "$SCRIPT_DIR/data/archive"
    mkdir -p "$SCRIPT_DIR/data/souls"
    log_ok "数据目录已创建"
}

setup_apikeys() {
    local apikeys_file="$SCRIPT_DIR/data/apikeys.json"
    if [ -f "$apikeys_file" ]; then
        log_warn "data/apikeys.json 已存在，跳过创建"
        return
    fi

    log_info "创建 API Key 配置文件..."
    cat > "$apikeys_file" << 'EOF'
{
  "_comment": "填入你的 API Key。如果只使用本地 LM Studio，可留空",
  "deepseek": "",
  "openai": "",
  "claude": ""
}
EOF
    log_ok "data/apikeys.json 已创建"
}

build_backend() {
    log_info "构建 Rust 后端（首次编译可能需要 5-10 分钟）..."
    cd "$SCRIPT_DIR"
    cargo build --package api --release
    log_ok "后端构建完成"
}

build_frontend() {
    log_info "构建 Next.js 前端..."
    cd "$SCRIPT_DIR/nextjs"
    pnpm install
    pnpm build
    log_ok "前端构建完成"
}

create_start_script() {
    local script_path="$SCRIPT_DIR/scripts/start-local.sh"
    mkdir -p "$SCRIPT_DIR/scripts"

    log_info "创建启动脚本..."
    cat > "$script_path" << 'SCRIPT'
#!/bin/bash
set -e
DIR="$(cd "$(dirname "$0")/.." && pwd)"
LAN_IP=$(ipconfig getifaddr en0 2>/dev/null || hostname -I 2>/dev/null | awk '{print $1}' || echo "")

cleanup() {
    echo ""
    echo "正在关闭服务..."
    kill $API_PID 2>/dev/null || true
    kill $FRONT_PID 2>/dev/null || true
    exit 0
}
trap cleanup SIGINT SIGTERM EXIT

echo "========================================"
echo "  Snake Skin"
echo "========================================"
echo ""

# Start API
echo "[1/2] 启动 API 服务 (0.0.0.0:3096)..."
cd "$DIR"
./target/release/api 2>&1 | sed 's/^/[API] /' &
API_PID=$!
sleep 2

# Wait for API
for i in $(seq 1 30); do
    if curl -s http://127.0.0.1:3096/api/v1/health > /dev/null 2>&1; then
        echo "[1/2] API 就绪"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "API 启动超时，请检查日志"
        exit 1
    fi
    sleep 1
done

# Start Frontend
echo "[2/2] 启动前端..."
cd "$DIR/nextjs"
NEXT_PUBLIC_API_URL="http://${LAN_IP:-127.0.0.1}:3096/api/v1" pnpm start 2>&1 | sed 's/^|[WEB] /' &
FRONT_PID=$!
sleep 3

echo ""
echo "========================================"
echo "  本地访问: http://localhost:3000"
if [ -n "$LAN_IP" ]; then
    echo "  局域网访问: http://${LAN_IP}:3000"
fi
echo "  API:      http://127.0.0.1:3096"
echo "  Ctrl+C    关闭所有服务"
echo "========================================"
echo ""

# Open browser
sleep 1
open http://localhost:3000 2>/dev/null || xdg-open http://localhost:3000 2>/dev/null || true

wait
SCRIPT

    chmod +x "$script_path"
    log_ok "启动脚本已创建: scripts/start-local.sh"
}

print_next_steps() {
    echo ""
    echo "============================================================"
    echo "  安装完成!"
    echo "============================================================"
    echo ""
    echo -e "${GREEN}方式一：本地算力（LM Studio）${NC}"
    echo "  1. 安装并启动 LM Studio: https://lmstudio.ai"
    echo "  2. 在 LM Studio 中加载模型"
    echo "  3. 打开 http://localhost:3000/models"
    echo "  4. 选择 LM Studio，填入模型名、API Key、端点地址"
    echo "  5. 点击「测试」验证连通性"
    echo "  6. 点击「设为活跃」切换 provider"
    echo ""
    echo -e "${GREEN}方式二：云端 API（DeepSeek / Claude / OpenAI）${NC}"
    echo "  1. 编辑 data/apikeys.json，填入 API Key"
    echo "  2. 打开 http://localhost:3000/models"
    echo "  3. 选择对应 provider，点击「设为活跃」"
    echo ""
    echo -e "${GREEN}启动${NC}"
    echo "  bash scripts/start-local.sh"
    echo ""
    echo -e "${GREEN}开发模式启动${NC}"
    echo "  # 终端 1: cd rust && cargo run --package api"
    echo "  # 终端 2: cd nextjs && pnpm dev"
    echo ""
}

# ============== Main ==============
print_banner
check_deps
setup_data_dir
setup_apikeys
build_backend
build_frontend
create_start_script
print_next_steps
