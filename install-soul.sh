#!/usr/bin/env bash
set -euo pipefail

echo "🧠 Soul Agent CLI — 安装脚本"
echo "=============================="
echo ""

# Check Rust
if ! command -v cargo &>/dev/null; then
    echo "❌ 需要 Rust 工具链。请先安装: https://rustup.rs"
    exit 1
fi

echo "✅ Rust 已安装: $(rustc --version)"

# Install soul CLI
echo ""
echo "📦 安装 soul 命令..."
cargo install --path rust/soul-agent

echo ""
echo "✅ 安装完成！"
echo ""
echo "用法:"
echo "  soul \"你的问题\""
echo "  soul \"task\" --souls 经济学家,HR总监 --mode conference"
echo "  echo \"task\" | soul"
echo ""
echo "需要将 soul 文件放入 data/souls/ 目录。"
echo "详见: rust/soul-agent/USAGE.md"
