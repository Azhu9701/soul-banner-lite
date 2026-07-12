# 创业沙盘 Godot 2D UI 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) for syntax tracking.

**Goal:** 完善创业沙盘 Godot 2D 前端 UI，实现生产线视图、财务报告、订货会、市场面板等核心交互。

**Architecture:** 所有 UI 控件使用 Godot 4 Control 节点，`Stretch` 自适应布局。`WebSocketManager` 单例负责通信，`MainGame` 主场景通过信号分发事件，各子场景各自更新。

**Tech Stack:** Godot 4.7, GDScript 2.0 (strict static typing), Control nodes

**设计文档:** `docs/superpowers/specs/2026-07-13-business-sandbox-godot-design.md`

---

## 文件结构

| 文件 | 职责 |
|------|------|
| `godot/scenes/MainGame.tscn` | 修改：增加子节点占位 |
| `godot/scripts/production_view.gd` | **新建**：生产线视窗 |
| `godot/scripts/inventory_dashboard.gd` | **新建**：库存看板 |
| `godot/scripts/popups/annual_report_popup.gd` | **新建**：年度报告弹窗 |
| `godot/scripts/popups/order_meeting_popup.gd` | **新建**：订货会弹窗 |
| `godot/scripts/popups/factory_dialog.gd` | **新建**：工厂管理弹窗 |
| `godot/scripts/market_panel.gd` | **新建**：市场状态面板 |
| `godot/scripts/main_game.gd` | 修改：集成各子面板 |

---

### Task 1: 生产线视窗 (ProductionView)

**Files:**
- Create: `godot/scripts/production_view.gd`

实现一个 `VBoxContainer` 包含 4 条生产线，每条显示：
- 产线名称（手工线/半自动/全自动）
- 状态标签（空闲/生产中/建设中/转产中）
- 进度条（`TextureProgressBar` 表示进度）
- 产品图标（4 种产品不同颜色）

```gdscript
# 输出数据格式：
func update(factories: Array) -> void:
    # factories[0].lines = [{id, line_type, status, product}]
    # status: "idle" | {"producing": "ben_ma"} | {"building": 2} | {"switching_to": [1, "meng_hu"]}
```

### Task 2: 库存看板 (InventoryDashboard)

**Files:**
- Create: `godot/scripts/inventory_dashboard.gd`

显示：
- 原料库存数量
- 在制品列表（每个产线的在制品）
- 成品库存（按产品分类）
- 应收账款列表（金额 + 账期）

### Task 3: 年度报告弹窗 (AnnualReportPopup)

**Files:**
- Create: `godot/scripts/popups/annual_report_popup.gd`

TabContainer 包含 4 个标签页：
1. **损益表**：收入、成本、毛利、费用、净利润
2. **资产负债表**：资产、负债、权益
3. **费用明细表**：各项费用汇总
4. **关键指标**：毛利率、ROE、资产负债率

### Task 4: 订货会弹窗 (OrderMeetingPopup)

**Files:**
- Create: `godot/scripts/popups/order_meeting_popup.gd`

- 显示可用订单卡片（产品、数量、单价、账期）
- 点击选中/取消订单
- 确认提交按钮

### Task 5: 市场状态面板 (MarketPanel)

**Files:**
- Create: `godot/scripts/market_panel.gd`

- 显示各市场名称、开发状态、排名
- 每年营销投入输入
- 市场预测数据展示

### Task 6: 工厂/贷款管理 (FactoryDialog + MainGame 改进)

**Files:**
- Create: `godot/scripts/popups/factory_dialog.gd`
- Modify: `godot/scripts/main_game.gd`

- 购买/租赁工厂弹窗
- 建设新产线选择
- 贷款类型和金额选择
- 贴现确认弹窗
