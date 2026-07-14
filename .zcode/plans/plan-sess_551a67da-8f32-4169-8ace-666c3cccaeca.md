# 创业沙盘 Godot 2D 全栈 — 最终实施计划

## 架构
```
Godot 2D  ←──WebSocket (JSON)──→  Rust（游戏引擎 + AI 竞争对手）
```
- Next.js **去掉**，AI 全部在 Rust 中实现
- 复用现有：AI 网关 (`GatewayRegistry`)、Soul 角色 (CEO/CFO/COO/CMO/顾问 已存在)、工具注册表、WebSocket 管理器、存储层

## 阶段 1：Rust 游戏引擎（核心规则）
**新增文件：** `rust/api/src/routes/business_sandbox.rs`
- 游戏状态结构：`BusinessGameState`（年/季度/现金/贷款/生产线/研发/库存/订单/市场）
- 时序引擎：年初决策 → 4 季度运营（严格 10 步）→ 年末结算
- 核心计算：贴现(1/14)、折旧(20%)、贷款利息、税(3%+20%)、行政管理费(1M/Q)
- 4 种产品（奔马/猛虎/飞鹰/天龙）和 3 种产线（手工/半自动/全自动）规则
- 订单与市场机制（营销激活、选单顺序、延期惩罚）
- 破产判定（股东权益<0 或现金流断裂）
- WebSocket 路由 `/ws/game/:game_id` 全双工通信

## 阶段 2：AI 竞争对手
- 复用已有商业 Soul 角色（CEO/CFO/COO/CMO/顾问）
- 通过 `GatewayRegistry` 并行调用 LLM
- 每年 AI 产出竞争策略（市场投入、定价、产能决策）→ 融入状态计算
- 工具：注册商业专用工具（`market_research`, `set_production`, `apply_loan` 等）

## 阶段 3：Godot 2D 前端
- Godot 4.2+，纯 GDScript，`Control` 节点 Stretch 自适应
- `WebSocketManager` 单例连接 Rust
- UI：顶部状态栏 | 中部 4 条生产线视窗 | 底部财务操作区
- 弹窗系统：年度报告（4 标签页）、订货会、贴现确认、破产

## 阶段 4：动画与发布
- `Tween` 动画：原料→产线→成品流动
- Web (HTML5) + 桌面双端导出

## 不修改现有代码
所有新代码在独立模块，不碰 `possession/`、`routes/possess.rs` 等现有仲裁庭逻辑