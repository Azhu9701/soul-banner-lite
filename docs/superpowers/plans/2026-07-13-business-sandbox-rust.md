# 创业沙盘 Rust 游戏引擎实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) for syntax tracking.

**Goal:** 在现有 Rust 项目中实现创业沙盘游戏引擎（状态管理 + 游戏循环 + 财务计算 + AI 竞争对手 + WebSocket 通信），不修改现有仲裁庭代码。

**架构:** Godot 2D ← WebSocket (JSON) → Rust 游戏引擎。Rust 维护唯一完整游戏状态，AI 竞争对手复用现有 GatewayRegistry / Soul 系统。

**Tech Stack:** Rust, Axum, tokio, sqlite (复用已有)

**设计文档:** `docs/superpowers/specs/2026-07-13-business-sandbox-godot-design.md`

---

## 文件结构

### 新增文件
| 文件 | 职责 |
|------|------|
| `rust/api/src/business_sandbox/mod.rs` | 模块入口，重导出 |
| `rust/api/src/business_sandbox/state.rs` | 游戏状态数据结构 |
| `rust/api/src/business_sandbox/engine.rs` | 游戏引擎（时序循环 + 规则执行） |
| `rust/api/src/business_sandbox/finance.rs` | 财务计算（贴现、折旧、贷款、税务） |
| `rust/api/src/business_sandbox/production.rs` | 生产与供应链（产品、产线、库存） |
| `rust/api/src/business_sandbox/market.rs` | 市场与订单（营销、选单、交付） |
| `rust/api/src/business_sandbox/ai_competitor.rs` | AI 竞争对手（复用 Gateway 调用 Soul） |
| `rust/api/src/business_sandbox/ws_handler.rs` | WebSocket 路由 + 事件广播 |
| `rust/api/src/business_sandbox/errors.rs` | 错误类型定义 |

### 修改文件
| 文件 | 改动 |
|------|------|
| `rust/api/src/main.rs` | 新增 `mod business_sandbox` + WS 路由挂载 |
| `rust/api/src/state.rs` | 新增 `BusinessSandboxManager` 字段 |
| `rust/api/src/routes/mod.rs` | 无改动（WS 路由在 main.rs 级别挂载） |

---

### Task 1: 游戏状态数据结构

**File:** `rust/api/src/business_sandbox/state.rs`

- [ ] **Step 1: 创建目录和 mod 文件**

```bash
mkdir -p rust/api/src/business_sandbox
```

Write `rust/api/src/business_sandbox/mod.rs`:

```rust
pub mod state;
pub mod engine;
pub mod finance;
pub mod production;
pub mod market;
pub mod ai_competitor;
pub mod ws_handler;
pub mod errors;
```

- [ ] **Step 2: 定义核心枚举和类型**

Write `rust/api/src/business_sandbox/state.rs`:

```rust
use serde::{Deserialize, Serialize};

/// 产品类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Product {
    BenMa,    // 奔马 - 基础产品
    MengHu,   // 猛虎 - 升级产品
    FeiYing,  // 飞鹰 - 中级产品
    TianLong, // 天龙 - 高级产品
}

impl Product {
    pub fn raw_material_cost(&self) -> u32 {
        match self {
            Product::BenMa => 1,
            Product::MengHu => 2,
            Product::FeiYing => 3,
            Product::TianLong => 4,
        }
    }

    pub fn production_cost(&self) -> u32 {
        match self {
            Product::BenMa | Product::MengHu => 1,
            Product::FeiYing | Product::TianLong => 2,
        }
    }

    pub fn name_cn(&self) -> &'static str {
        match self {
            Product::BenMa => "奔马",
            Product::MengHu => "猛虎",
            Product::FeiYing => "飞鹰",
            Product::TianLong => "天龙",
        }
    }
}

/// 生产线类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineType {
    Manual,     // 手工线
    SemiAuto,   // 半自动线
    FullAuto,   // 全自动线
}

impl LineType {
    pub fn build_time(&self) -> u32 { // 季度数
        match self {
            LineType::Manual => 1,
            LineType::SemiAuto => 4,
            LineType::FullAuto => 4,
        }
    }

    pub fn build_cost_per_quarter(&self) -> u32 {
        match self {
            LineType::Manual => 4,
            LineType::SemiAuto => 2,
            LineType::FullAuto => 4,
        }
    }

    pub fn total_build_cost(&self) -> u32 {
        match self {
            LineType::Manual => 4,
            LineType::SemiAuto => 8,
            LineType::FullAuto => 16,
        }
    }

    pub fn switch_time(&self) -> u32 {
        match self {
            LineType::Manual => 0,
            LineType::SemiAuto => 1,
            LineType::FullAuto => 2,
        }
    }

    pub fn switch_cost(&self) -> u32 {
        match self {
            LineType::Manual => 0,
            LineType::SemiAuto => 1,
            LineType::FullAuto => 4, // 2M/季 × 2季
        }
    }

    pub fn salvage_value(&self) -> u32 {
        match self {
            LineType::Manual => 2,
            LineType::SemiAuto => 3,
            LineType::FullAuto => 6,
        }
    }
}

/// 生产线状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LineStatus {
    Idle,               // 空闲
    Producing(Product), // 生产中
    Building(u32),      // 建设中（剩余季度数）
    Switching(u32),     // 转产中（剩余季度数，目标产品）
    SwitchingTo(u32, Product), // 转产中（剩余季度，目标产品）
}

/// 一条生产线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionLine {
    pub id: u32,
    pub line_type: LineType,
    pub status: LineStatus,
    pub product: Option<Product>, // 当前配置生产的产品
}

/// 工厂
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factory {
    pub id: String,
    pub name: String,
    pub capacity: u32,     // 可容纳产线数
    pub value: u32,        // 固定资产价值(M)
    pub lines: Vec<ProductionLine>,
}

/// 产品研发状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRDA {
    pub product: Product,
    pub progress: u32,   // 已研发季度数
    pub total: u32,      // 需要总季度数
    pub completed: bool,
}

/// 应收账款
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountReceivable {
    pub amount: u32,
    pub due_quarters: u32, // 剩余账期（季度数）
}

/// 贷款
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub amount: u32,
    pub remaining_quarters: u32, // 长贷：剩余年数×4；短贷：剩余季度数
    pub annual_interest_rate: f64, // 年利率
}

/// 长期贷款槽位（按年份）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermLoanSlot {
    pub year: u32,  // 第几年
    pub amount: u32,
    pub active: bool,
}

/// 市场
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub name: String,
    pub developed: bool,
    pub rank: u32,             // 公司在该市场的排名
    pub last_year_sales: u32,  // 去年在该市场的销售额
}

/// 订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub product: Product,
    pub quantity: u32,     // 批次数量
    pub unit_price: u32,   // 单价
    pub account_period: u32, // 账期（季度数）
    pub delivered: bool,
    pub urgent: bool,
}

/// 竞标策略（由玩家填写）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiddingStrategy {
    pub market_name: String,
    pub marketing_spend: u32,  // 营销投入（M）
}

/// 完整游戏状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessGameState {
    pub game_year: u32,
    pub game_quarter: u32,  // 1-4
    pub cash: u32,
    pub factories: Vec<Factory>,
    pub products_rd: Vec<ProductRDA>,
    pub raw_material_orders: u32,  // 待交付的原料订单数
    pub raw_material_inventory: u32, // 原料库存（个数）
    pub work_in_progress: Vec<WIPItem>,
    pub finished_goods: Vec<FinishedGoods>,
    pub accounts_receivable: Vec<AccountReceivable>,
    pub long_term_loans: Vec<LongTermLoanSlot>,
    pub short_term_loans: Vec<Loan>,
    pub markets: Vec<Market>,
    pub orders: Vec<Order>,
    pub bidding_strategies: Vec<BiddingStrategy>,
    pub phase: u32, // 1-4
    pub game_over: bool,
    pub game_over_reason: Option<String>,
}

/// 在制品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WIPItem {
    pub line_id: u32,
    pub product: Product,
    pub progress: u32, // 生产进度（季度数）
}

/// 成品库存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedGoods {
    pub product: Product,
    pub quantity: u32,
}

/// 年度财务报表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualReport {
    pub year: u32,
    pub income_statement: IncomeStatement,
    pub balance_sheet: BalanceSheet,
    pub expense_sheet: ExpenseSheet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub revenue: u32,
    pub cost_of_goods_sold: u32,
    pub gross_profit: i32,
    pub sales_expense: u32,
    pub admin_expense: u32,
    pub rd_expense: u32,
    pub depreciation: u32,
    pub operating_profit: i32,
    pub interest_expense: u32,
    pub discount_fee: u32,
    pub tax: u32,
    pub net_profit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub cash: u32,
    pub accounts_receivable: u32,
    pub raw_material: u32,
    pub work_in_process: u32,
    pub finished_goods: u32,
    pub total_current_assets: u32,
    pub factory: u32,
    pub production_lines: u32,
    pub total_fixed_assets: u32,
    pub total_assets: u32,
    pub long_term_loans: u32,
    pub short_term_loans: u32,
    pub total_liabilities: u32,
    pub equity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseSheet {
    pub new_market_investment: u32,
    pub product_rd_investment: u32,
    pub quarterly_admin_fees: u32,
    pub factory_rent: u32,
    pub line_maintenance: u32,
    pub product_switch_fees: u32,
    pub sales_expenses: u32,
    pub depreciation: u32,
    pub interest_and_discount: u32,
    pub tax: u32,
}
```

- [ ] **Step 3: 定义 WebSocket 消息类型**

Add to `rust/api/src/business_sandbox/state.rs`:

```rust
/// 服务端推送的游戏事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum GameEvent {
    #[serde(rename = "state_update")]
    StateUpdate(Box<BusinessGameState>),
    #[serde(rename = "phase_change")]
    PhaseChange { year: u32, quarter: u32, phase: String },
    #[serde(rename = "ask_decision")]
    AskDecision { year: u32, quarter: u32, decision_type: String },
    #[serde(rename = "annual_report")]
    AnnualReport(Box<AnnualReport>),
    #[serde(rename = "order_meeting")]
    OrderMeeting { market: String, available_orders: Vec<Order> },
    #[serde(rename = "game_over")]
    GameOver { reason: String },
    #[serde(rename = "message")]
    Message(String),
}

/// 客户端发送的操作
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action")]
pub enum PlayerAction {
    #[serde(rename = "start_game")]
    StartGame,
    #[serde(rename = "submit_bidding")]
    SubmitBidding { strategies: Vec<BiddingStrategy> },
    #[serde(rename = "select_order")]
    SelectOrder { order_ids: Vec<String> },
    #[serde(rename = "make_decision")]
    MakeDecision { decisions: Vec<DecisionItem> },
    #[serde(rename = "next_quarter")]
    NextQuarter,
    #[serde(rename = "discount_receivable")]
    DiscountReceivable { amount: u32 },
    #[serde(rename = "take_loan")]
    TakeLoan { loan_type: String, amount: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionItem {
    pub key: String,
    pub value: serde_json::Value,
}
```

- [ ] **Step 4: 定义错误类型**

Write `rust/api/src/business_sandbox/errors.rs`:

```rust
use std::fmt;

#[derive(Debug)]
pub enum GameError {
    GameNotFound,
    InvalidAction(String),
    InsufficientFunds(u32, u32), // need, have
    RuleViolation(String),
    Internal(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::GameNotFound => write!(f, "Game not found"),
            GameError::InvalidAction(msg) => write!(f, "Invalid action: {}", msg),
            GameError::InsufficientFunds(need, have) => write!(f, "Insufficient funds: need {}, have {}", need, have),
            GameError::RuleViolation(msg) => write!(f, "Rule violation: {}", msg),
            GameError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for GameError {}
```

- [ ] **Step 5: Commit**

```bash
git add rust/api/src/business_sandbox/
git commit -m "feat(business-sandbox): 定义游戏状态数据结构和消息类型"
```

---

### Task 2: 财务计算模块

**Files:**
- Create: `rust/api/src/business_sandbox/finance.rs`

- [ ] **Step 1: 实现财务计算函数**

```rust
use crate::business_sandbox::state::*;

/// 计算贴现手续费 = 金额 × 1/14，四舍五入取整
pub fn calc_discount_fee(amount: u32) -> u32 {
    ((amount as f64) / 14.0).round() as u32
}

/// 计算固定资产折旧 = 原值 × 20%，四舍五入取整
pub fn calc_depreciation(value: u32) -> u32 {
    ((value as f64) * 0.20).round() as u32
}

/// 计算长期贷款年利息
pub fn calc_long_term_interest(principal: u32) -> u32 {
    ((principal as f64) * 0.05).round() as u32
}

/// 计算短期贷款季度利息（年息10% ÷ 4）
pub fn calc_short_term_quarterly_interest(principal: u32) -> u32 {
    ((principal as f64) * 0.10 / 4.0).round() as u32
}

/// 计算营业税 = 销售额 × 3%
pub fn calc_sales_tax(revenue: u32) -> u32 {
    ((revenue as f64) * 0.03).round() as u32
}

/// 计算所得税 = 税前利润 × 20%（利润为负则免）
pub fn calc_income_tax(profit_before_tax: i32) -> u32 {
    if profit_before_tax <= 0 {
        return 0;
    }
    ((profit_before_tax as f64) * 0.20).round() as u32
}

/// 计算贷款总额度 = 上年度净资产 × 4（长贷2倍 + 短贷2倍）
pub fn calc_loan_limit(net_assets: i32) -> (u32, u32) {
    let base = if net_assets > 0 { net_assets as u32 } else { 0 };
    let long_term_limit = base * 2;
    let short_term_limit = base * 2;
    (long_term_limit, short_term_limit)
}

/// 验证贷款额度的合法性（最低20M，20M倍数）
pub fn validate_loan_amount(amount: u32) -> Result<u32, String> {
    if amount < 20 {
        return Err("最低起贷20M".into());
    }
    if amount % 20 != 0 {
        return Err("只能贷20M的倍数".into());
    }
    Ok(amount)
}

/// 计算公司估值（第三阶段使用）
pub fn calc_company_valuation(
    equity: u32,
    last_year_net_profit: i32,
    this_year_net_profit: i32,
    recent_3yr_investment: u32,
) -> u32 {
    let last_profit = if last_year_net_profit > 0 { last_year_net_profit as u32 } else { 0 };
    let this_profit = if this_year_net_profit > 0 { this_year_net_profit as u32 } else { 0 };
    equity + last_profit + this_profit * 2 + recent_3yr_investment / 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discount_fee() {
        // 1/14 of 14M = 1M
        assert_eq!(calc_discount_fee(14), 1);
        // 1/14 of 28M = 2M
        assert_eq!(calc_discount_fee(28), 2);
        // 1/14 of 13M ≈ 0.93 → 1 (四舍五入)
        assert_eq!(calc_discount_fee(13), 1);
    }

    #[test]
    fn test_depreciation() {
        // 20% of 20M = 4M
        assert_eq!(calc_depreciation(20), 4);
        // 20% of 10M = 2M
        assert_eq!(calc_depreciation(10), 2);
        // 20% of 16M = 3.2 → 3
        assert_eq!(calc_depreciation(16), 3);
    }

    #[test]
    fn test_loan_interest() {
        // 5% of 40M = 2M
        assert_eq!(calc_long_term_interest(40), 2);
        // 10%/4 of 40M = 1M
        assert_eq!(calc_short_term_quarterly_interest(40), 1);
    }

    #[test]
    fn test_sales_tax() {
        // 3% of 100M = 3M
        assert_eq!(calc_sales_tax(100), 3);
    }

    #[test]
    fn test_income_tax() {
        // 20% of 50M = 10M
        assert_eq!(calc_income_tax(50), 10);
        // Negative profit = 0 tax
        assert_eq!(calc_income_tax(-10), 0);
    }

    #[test]
    fn test_loan_validation() {
        assert!(validate_loan_amount(20).is_ok());
        assert!(validate_loan_amount(40).is_ok());
        assert!(validate_loan_amount(10).is_err()); // below minimum
        assert!(validate_loan_amount(30).is_err()); // not multiple of 20
    }
}
```

- [ ] **Step 2: Run tests to verify**

```bash
cd rust && cargo test --package api business_sandbox::finance 2>&1 | head -30
```

Expected: ALL TESTS PASS

- [ ] **Step 3: Commit**

```bash
git add rust/api/src/business_sandbox/finance.rs
git commit -m "feat(business-sandbox): 实现财务计算模块（贴现/折旧/贷款/税务）"
```

---

### Task 3: 生产与供应链模块

**File:** `rust/api/src/business_sandbox/production.rs`

- [ ] **Step 1: 实现产品研发和生产线管理**

```rust
use crate::business_sandbox::state::*;

/// 产品研发所需总季度数
pub fn product_rd_total_quarters(product: Product, has_feijing: bool) -> u32 {
    match product {
        Product::BenMa => 0, // 已有
        Product::MengHu => 6,
        Product::FeiYing => 8,
        Product::TianLong => {
            if has_feijing {
                8 + 6 - 5 // 有飞鹰则缩短5季度，但总研发不减?
                // PDF原文: "开发周期可以缩短5个季度，相应的开发资金也少投入5M"
                // 需要根据实际PDF规则判断
                8 + 6 - 5 // 简化: 基础8+6=14，缩短5=9
            } else {
                8 + 6 // 基础=14
            }
        }
    }
}

/// 产品研发总费用（每季度1M）
pub fn product_rd_total_cost(quarters: u32) -> u32 {
    quarters
}

/// 原材料成本
pub fn raw_material_cost(product: Product) -> u32 {
    product.raw_material_cost()
}

/// 生产费用
pub fn production_cost(product: Product) -> u32 {
    product.production_cost()
}

/// 生产线建设费用汇总
pub fn total_line_build_cost(line_type: LineType) -> u32 {
    line_type.total_build_cost()
}

/// 计算生产线当前固定资产残值
pub fn line_depreciated_value(original_value: u32, years: u32) -> u32 {
    let mut value = original_value;
    for _ in 0..years {
        let dep = crate::business_sandbox::finance::calc_depreciation(value);
        if dep >= value {
            value = 0;
        } else {
            value -= dep;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_rd_time() {
        // 猛虎研发6季度
        assert_eq!(product_rd_total_quarters(Product::MengHu, false), 6);
        // 飞鹰研发8季度
        assert_eq!(product_rd_total_quarters(Product::FeiYing, false), 8);
    }

    #[test]
    fn test_raw_material_cost() {
        assert_eq!(raw_material_cost(Product::BenMa), 1);
        assert_eq!(raw_material_cost(Product::TianLong), 4);
    }

    #[test]
    fn test_production_cost() {
        assert_eq!(production_cost(Product::BenMa), 1);
        assert_eq!(production_cost(Product::FeiYing), 2);
    }

    #[test]
    fn test_line_build_cost() {
        assert_eq!(total_line_build_cost(LineType::Manual), 4);
        assert_eq!(total_line_build_cost(LineType::SemiAuto), 8);
        assert_eq!(total_line_build_cost(LineType::FullAuto), 16);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd rust && cargo test --package api business_sandbox::production 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add rust/api/src/business_sandbox/production.rs
git commit -m "feat(business-sandbox): 实现生产与供应链模块"
```

---

### Task 4: 市场与订单模块

**File:** `rust/api/src/business_sandbox/market.rs`

- [ ] **Step 1: 实现市场与订单逻辑**

```rust
use crate::business_sandbox::state::*;

/// 计算营销投入激活的订单数（每1M激活1张）
pub fn calc_activated_orders(marketing_spend: u32, max_orders: u32) -> u32 {
    marketing_spend.min(max_orders)
}

/// 选单顺序：按上年度市场排名 + 营销投入排序
pub fn calc_selection_order(companies: &[(u32, u32)]) -> Vec<usize> {
    // companies: [(last_year_sales_or_rank, marketing_spend), ...]
    // 返回排序后的索引：市场第1优先按排名，其余按营销投入
    let mut indices: Vec<usize> = (0..companies.len()).collect();
    indices.sort_by(|&a, &b| {
        let (rank_a, spend_a) = companies[a];
        let (rank_b, spend_b) = companies[b];
        // 排名越靠前（数字小）越优先
        if rank_a == 1 && rank_b != 1 {
            return std::cmp::Ordering::Less;
        }
        if rank_b == 1 && rank_a != 1 {
            return std::cmp::Ordering::Greater;
        }
        // 其余按营销投入降序
        spend_b.cmp(&spend_a)
    });
    indices
}

/// 计算延期交付惩罚（支付75%）
pub fn calc_late_delivery_penalty(original_amount: u32) -> u32 {
    ((original_amount as f64) * 0.75).round() as u32
}

/// 生成年度市场报告摘要
pub fn generate_market_summary(markets: &[Market]) -> Vec<MarketSummary> {
    markets.iter().map(|m| {
        MarketSummary {
            name: m.name.clone(),
            developed: m.developed,
            rank: m.rank,
            last_year_sales: m.last_year_sales,
        }
    }).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSummary {
    pub name: String,
    pub developed: bool,
    pub rank: u32,
    pub last_year_sales: u32,
}
```

- [ ] **Step 2: Commit**

```bash
git add rust/api/src/business_sandbox/market.rs
git commit -m "feat(business-sandbox): 实现市场与订单模块"
```

---

### Task 5: 游戏引擎（核心时序循环）

**File:** `rust/api/src/business_sandbox/engine.rs`

- [ ] **Step 1: 定义 GameManager**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::business_sandbox::state::*;
use crate::business_sandbox::errors::GameError;

/// 游戏实例管理器
pub struct GameManager {
    games: Mutex<HashMap<String, GameInstance>>,
}

pub struct GameInstance {
    pub state: BusinessGameState,
    pub annual_reports: Vec<AnnualReport>,
    pub current_phase: String, // "year_start" | "quarter_ops" | "year_end" | "waiting"
    pub pending_decision: Option<String>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: Mutex::new(HashMap::new()),
        }
    }

    /// 创建新游戏（初始化状态）
    pub async fn create_game(&self, game_id: &str) -> Result<(), GameError> {
        let mut games = self.games.lock().await;
        let state = Self::initial_state();
        games.insert(game_id.to_string(), GameInstance {
            state,
            annual_reports: Vec::new(),
            current_phase: "year_start".into(),
            pending_decision: Some("bidding".into()),
        });
        Ok(())
    }

    /// 获取游戏状态
    pub async fn get_state(&self, game_id: &str) -> Result<BusinessGameState, GameError> {
        let games = self.games.lock().await;
        games.get(game_id)
            .map(|g| g.state.clone())
            .ok_or(GameError::GameNotFound)
    }

    /// 处理玩家操作
    pub async fn handle_action(
        &self,
        game_id: &str,
        action: PlayerAction,
    ) -> Result<Vec<GameEvent>, GameError> {
        let mut games = self.games.lock().await;
        let instance = games.get_mut(game_id)
            .ok_or(GameError::GameNotFound)?;

        match action {
            PlayerAction::StartGame => self.handle_start_game(instance),
            PlayerAction::SubmitBidding { strategies } => {
                self.handle_submit_bidding(instance, strategies)
            }
            PlayerAction::NextQuarter => self.handle_next_quarter(instance),
            PlayerAction::DiscountReceivable { amount } => {
                self.handle_discount(instance, amount)
            }
            PlayerAction::TakeLoan { loan_type, amount } => {
                self.handle_take_loan(instance, &loan_type, amount)
            }
            _ => Err(GameError::InvalidAction("未实现的操".into())),
        }
    }

    fn initial_state() -> BusinessGameState {
        BusinessGameState {
            game_year: 1,
            game_quarter: 1,
            cash: 12, // 初始现金12M
            factories: vec![Factory {
                id: "old_factory".into(),
                name: "老工厂".into(),
                capacity: 4,
                value: 20,
                lines: vec![ProductionLine {
                    id: 1,
                    line_type: LineType::Manual,
                    status: LineStatus::Producing(Product::BenMa),
                    product: Some(Product::BenMa),
                }],
            }],
            products_rd: vec![],
            raw_material_orders: 2,
            raw_material_inventory: 8,
            work_in_progress: vec![WIPItem {
                line_id: 1,
                product: Product::BenMa,
                progress: 1,
            }],
            finished_goods: vec![FinishedGoods {
                product: Product::BenMa,
                quantity: 4,
            }],
            accounts_receivable: vec![AccountReceivable {
                amount: 12,
                due_quarters: 4,
            }],
            long_term_loans: vec![],
            short_term_loans: vec![],
            markets: vec![Market {
                name: "平城".into(),
                developed: true,
                rank: 1,
                last_year_sales: 0,
            }],
            orders: vec![],
            bidding_strategies: vec![],
            phase: 1,
            game_over: false,
            game_over_reason: None,
        }
    }

    fn handle_start_game(&self, instance: &mut GameInstance) -> Result<Vec<GameEvent>, GameError> {
        instance.current_phase = "year_start".into();
        instance.pending_decision = Some("bidding".into());
        Ok(vec![
            GameEvent::StateUpdate(Box::new(instance.state.clone())),
            GameEvent::PhaseChange {
                year: instance.state.game_year,
                quarter: 0,
                phase: "year_start".into(),
            },
            GameEvent::AskDecision {
                year: instance.state.game_year,
                quarter: 0,
                decision_type: "bidding".into(),
            },
        ])
    }

    fn handle_submit_bidding(
        &self,
        instance: &mut GameInstance,
        strategies: Vec<BiddingStrategy>,
    ) -> Result<Vec<GameEvent>, GameError> {
        // 验证现金流
        let total_spend: u32 = strategies.iter().map(|s| s.marketing_spend).sum();
        if total_spend > instance.state.cash {
            return Err(GameError::InsufficientFunds(total_spend, instance.state.cash));
        }

        instance.state.bidding_strategies = strategies.clone();
        instance.state.cash -= total_spend;

        // 将销售费用记入费用
        instance.current_phase = "quarter_ops".into();
        instance.pending_decision = None;

        let mut events = vec![
            GameEvent::StateUpdate(Box::new(instance.state.clone())),
            GameEvent::Message(format!("营销费用已扣除{}M", total_spend)),
        ];

        // 开始季度循环
        events.push(GameEvent::PhaseChange {
            year: instance.state.game_year,
            quarter: 1,
            phase: "quarter_ops".into(),
        });

        Ok(events)
    }

    fn handle_next_quarter(&self, instance: &mut GameInstance) -> Result<Vec<GameEvent>, GameError> {
        let mut events = Vec::new();

        // 执行季度10步
        self.execute_quarter(instance, &mut events)?;

        // 检查是否到年末
        if instance.state.game_quarter >= 4 {
            // 执行年末结算
            self.execute_year_end(instance, &mut events)?;
            instance.state.game_year += 1;
            instance.state.game_quarter = 0;
            instance.current_phase = "year_start".into();

            events.push(GameEvent::PhaseChange {
                year: instance.state.game_year,
                quarter: 0,
                phase: "year_end".into(),
            });

            // 生成年度报告
            let report = self.generate_annual_report(instance);
            instance.annual_reports.push(report.clone());
            events.push(GameEvent::AnnualReport(Box::new(report)));

            // 破产检查
            if instance.state.cash > 0 || instance.state.factories.iter().any(|f| f.value > 0) {
                events.push(GameEvent::AskDecision {
                    year: instance.state.game_year,
                    quarter: 0,
                    decision_type: "bidding".into(),
                });
            }
        } else {
            instance.state.game_quarter += 1;
            events.push(GameEvent::PhaseChange {
                year: instance.state.game_year,
                quarter: instance.state.game_quarter,
                phase: "quarter_ops".into(),
            });
            events.push(GameEvent::AskDecision {
                year: instance.state.game_year,
                quarter: instance.state.game_quarter,
                decision_type: "next_quarter".into(),
            });
        }

        events.push(GameEvent::StateUpdate(Box::new(instance.state.clone())));
        Ok(events)
    }

    fn execute_quarter(&self, instance: &mut GameInstance, events: &mut Vec<GameEvent>) -> Result<(), GameError> {
        let s = &mut instance.state;

        // Step 1: 更新应收账款
        for ar in &mut s.accounts_receivable {
            if ar.due_quarters > 0 {
                ar.due_quarters -= 1;
                if ar.due_quarters == 0 {
                    s.cash += ar.amount;
                }
            }
        }
        s.accounts_receivable.retain(|ar| ar.due_quarters > 0);

        // Step 2: 更新短贷利息
        for loan in &mut s.short_term_loans {
            let interest = crate::business_sandbox::finance::calc_short_term_quarterly_interest(loan.amount);
            if s.cash >= interest {
                s.cash -= interest;
            } else {
                // 现金流断裂
                s.game_over = true;
                s.game_over_reason = Some("现金流断裂(Q2 短贷利息)".into());
                events.push(GameEvent::GameOver {
                    reason: s.game_over_reason.clone().unwrap(),
                });
                return Ok(());
            }
            if loan.remaining_quarters > 0 {
                loan.remaining_quarters -= 1;
            }
        }
        s.short_term_loans.retain(|l| l.remaining_quarters > 0);

        // Step 3: 产品研发推进
        for rd in &mut s.products_rd {
            if !rd.completed {
                rd.progress += 1;
                if rd.progress >= rd.total {
                    rd.completed = true;
                    events.push(GameEvent::Message(format!("{}研发完成！", rd.product.name_cn())));
                }
            }
        }

        // Step 4: 供应商交货
        if s.raw_material_orders > 0 {
            // 每个订单交付1个原料，需付费
            let cost = s.raw_material_orders; // 简化：1M/个
            if s.cash >= cost {
                s.cash -= cost;
                s.raw_material_inventory += s.raw_material_orders;
                s.raw_material_orders = 0;
            }
        }

        // Step 5: 发出新原料订单（简化：自动按需）
        // Step 6: 更新生产状态
        for wip in &mut s.work_in_progress {
            wip.progress += 1;
        }
        // 完成的生产转入成品库
        let mut completed: Vec<WIPItem> = Vec::new();
        s.work_in_progress.retain(|wip| {
            if wip.progress >= 2 { // 简化：2季度生产周期
                completed.push(wip.clone());
                false
            } else {
                true
            }
        });
        for item in &completed {
            let existing = s.finished_goods.iter_mut()
                .find(|fg| fg.product == item.product);
            if let Some(fg) = existing {
                fg.quantity += 1;
            } else {
                s.finished_goods.push(FinishedGoods {
                    product: item.product,
                    quantity: 1,
                });
            }
        }

        // Step 7: 生产线建设推进
        for factory in &mut s.factories {
            for line in &mut factory.lines {
                match &line.status {
                    LineStatus::Building(remaining) => {
                        let new_remaining = *remaining - 1;
                        if new_remaining == 0 {
                            line.status = LineStatus::Idle;
                            events.push(GameEvent::Message(format!("产线{}建设完成", line.id)));
                        } else {
                            line.status = LineStatus::Building(new_remaining);
                        }
                    }
                    LineStatus::SwitchingTo(remaining, target) => {
                        let new_remaining = *remaining - 1;
                        if new_remaining == 0 {
                            line.product = Some(*target);
                            line.status = LineStatus::Idle;
                            events.push(GameEvent::Message(format!("产线{}转产{}完成", line.id, target.name_cn())));
                        } else {
                            line.status = LineStatus::SwitchingTo(new_remaining, *target);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Step 8: 新生产（简化：生产线空闲则自动开始）
        for factory in &mut s.factories {
            for line in &mut factory.lines {
                if line.status == LineStatus::Idle {
                    if let Some(product) = line.product {
                        if s.raw_material_inventory >= 1 {
                            s.raw_material_inventory -= 1;
                            let cost = product.production_cost();
                            if s.cash >= cost {
                                s.cash -= cost;
                                line.status = LineStatus::Producing(product);
                                s.work_in_progress.push(WIPItem {
                                    line_id: line.id,
                                    product,
                                    progress: 0,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Step 9: 订单交付
        for order in &mut s.orders {
            if !order.delivered {
                let fg = s.finished_goods.iter()
                    .find(|g| g.product == order.product && g.quantity >= order.quantity);
                if let Some(goods) = fg {
                    goods.quantity -= order.quantity;
                    order.delivered = true;
                    let amount = order.quantity * order.unit_price;
                    s.accounts_receivable.push(AccountReceivable {
                        amount,
                        due_quarters: order.account_period,
                    });
                }
            }
        }

        // Step 10: 行政管理费
        if s.cash >= 1 {
            s.cash -= 1;
        } else {
            s.game_over = true;
            s.game_over_reason = Some("现金流断裂(Q10 行政管理费)".into());
            events.push(GameEvent::GameOver {
                reason: s.game_over_reason.clone().unwrap(),
            });
        }

        Ok(())
    }

    fn execute_year_end(&self, instance: &mut GameInstance, events: &mut Vec<GameEvent>) -> Result<(), GameError> {
        let s = &mut instance.state;

        // 1. 生产线管理费：1M/条
        for factory in &mut s.factories {
            for line in &mut factory.lines {
                if line.status != LineStatus::Building(0) && line.status != LineStatus::Building(0) {
                    // 建成投产的线收费
                    if s.cash >= 1 {
                        s.cash -= 1;
                    }
                }
            }
        }

        // 2. 固定资产折旧 20%
        for factory in &mut s.factories {
            let dep = crate::business_sandbox::finance::calc_depreciation(factory.value);
            if dep >= factory.value {
                factory.value = 0;
            } else {
                factory.value -= dep;
            }
            for line in &mut factory.lines {
                match &line.status {
                    LineStatus::Idle | LineStatus::Producing(_) => {
                        let line_value = line.line_type.salvage_value().max(1);
                        let line_dep = crate::business_sandbox::finance::calc_depreciation(line_value);
                        // 简化：暂时不追踪每条线的独立价值
                    }
                    _ => {}
                }
            }
        }

        // 3. 长期贷款更新
        for loan in &mut s.long_term_loans {
            if loan.active {
                let interest = crate::business_sandbox::finance::calc_long_term_interest(loan.amount);
                if s.cash >= interest {
                    s.cash -= interest;
                }
                if loan.remaining_quarters > 0 {
                    loan.remaining_quarters -= 1;
                }
                if loan.remaining_quarters == 0 {
                    // 到期还本
                    if s.cash >= loan.amount {
                        s.cash -= loan.amount;
                        loan.active = false;
                    }
                }
            }
        }

        // 4. 税款（简化版，完整版在Task 6实现）
        // 5. 破产检查
        let total_equity = self.calc_equity(s);
        if total_equity < 0 {
            s.game_over = true;
            s.game_over_reason = Some("股东权益为负".into());
            events.push(GameEvent::GameOver {
                reason: "股东权益<0，公司破产".into(),
            });
        }

        // 更新阶段
        if s.game_year >= 5 && s.phase < 2 { s.phase = 2; }
        if s.game_year >= 8 && s.phase < 3 { s.phase = 3; }

        Ok(())
    }

    fn calc_equity(&self, s: &BusinessGameState) -> i32 {
        let total_assets = s.cash as i32
            + s.accounts_receivable.iter().map(|a| a.amount as i32).sum::<i32>()
            + s.raw_material_inventory as i32
            + s.finished_goods.iter().map(|g| g.quantity as i32 * g.product.raw_material_cost() as i32 * 2).sum::<i32>()
            + s.factories.iter().map(|f| f.value as i32).sum::<i32>();

        let total_liabilities = s.long_term_loans.iter().map(|l| l.amount as i32).sum::<i32>()
            + s.short_term_loans.iter().map(|l| l.amount as i32).sum::<i32>();

        total_assets - total_liabilities
    }

    fn generate_annual_report(&self, instance: &GameInstance) -> AnnualReport {
        let s = &instance.state;
        AnnualReport {
            year: s.game_year,
            income_statement: IncomeStatement {
                revenue: 0,
                cost_of_goods_sold: 0,
                gross_profit: 0,
                sales_expense: 0,
                admin_expense: s.game_quarter as u32,
                rd_expense: 0,
                depreciation: 0,
                operating_profit: 0,
                interest_expense: 0,
                discount_fee: 0,
                tax: 0,
                net_profit: 0,
            },
            balance_sheet: BalanceSheet {
                cash: s.cash,
                accounts_receivable: s.accounts_receivable.iter().map(|a| a.amount).sum(),
                raw_material: s.raw_material_inventory,
                work_in_process: 0,
                finished_goods: s.finished_goods.iter().map(|g| g.quantity).sum(),
                total_current_assets: 0,
                factory: s.factories.iter().map(|f| f.value).sum(),
                production_lines: 0,
                total_fixed_assets: 0,
                total_assets: 0,
                long_term_loans: s.long_term_loans.iter().map(|l| l.amount).sum(),
                short_term_loans: s.short_term_loans.iter().map(|l| l.amount).sum(),
                total_liabilities: 0,
                equity: 0,
            },
            expense_sheet: ExpenseSheet {
                new_market_investment: 0,
                product_rd_investment: 0,
                quarterly_admin_fees: s.game_quarter as u32,
                factory_rent: 0,
                line_maintenance: 0,
                product_switch_fees: 0,
                sales_expenses: 0,
                depreciation: 0,
                interest_and_discount: 0,
                tax: 0,
            },
        }
    }

    fn handle_discount(&self, instance: &mut GameInstance, amount: u32) -> Result<Vec<GameEvent>, GameError> {
        let fee = crate::business_sandbox::finance::calc_discount_fee(amount);
        instance.state.cash += amount - fee;
        instance.state.accounts_receivable.retain(|ar| {
            if ar.amount == amount {
                false
            } else {
                // 只能整笔贴现
                true
            }
        });
        Ok(vec![
            GameEvent::Message(format!("贴现{}M，手续费{}M，实收{}M", amount, fee, amount - fee)),
            GameEvent::StateUpdate(Box::new(instance.state.clone())),
        ])
    }

    fn handle_take_loan(&self, instance: &mut GameInstance, loan_type: &str, amount: u32) -> Result<Vec<GameEvent>, GameError> {
        crate::business_sandbox::finance::validate_loan_amount(amount)
            .map_err(|e| GameError::RuleViolation(e))?;

        instance.state.cash += amount;

        match loan_type {
            "long" => {
                instance.state.long_term_loans.push(LongTermLoanSlot {
                    year: instance.state.game_year,
                    amount,
                    active: true,
                });
            }
            "short" => {
                instance.state.short_term_loans.push(Loan {
                    amount,
                    remaining_quarters: 4,
                    annual_interest_rate: 0.10,
                });
            }
            _ => return Err(GameError::InvalidAction("未知贷款类型".into())),
        }

        Ok(vec![
            GameEvent::Message(format!("{}贷款{}M到账", if loan_type == "long" { "长期" } else { "短期" }, amount)),
            GameEvent::StateUpdate(Box::new(instance.state.clone())),
        ])
    }
}
```

- [ ] **Step 2: Add to mod.rs**

```rust
// in rust/api/src/business_sandbox/mod.rs
pub mod engine;
```

- [ ] **Step 3: Commit**

```bash
git add rust/api/src/business_sandbox/engine.rs
git add rust/api/src/business_sandbox/mod.rs
git commit -m "feat(business-sandbox): 实现游戏引擎核心时序循环"
```

---

### Task 6: WebSocket 路由和事件广播

**Files:**
- Modify: `rust/api/src/ws.rs`
- Modify: `rust/api/src/main.rs`
- Modify: `rust/api/src/state.rs`
- Create: `rust/api/src/business_sandbox/ws_handler.rs`

- [ ] **Step 1: 创建 WS handler**

Write `rust/api/src/business_sandbox/ws_handler.rs`:

```rust
use std::sync::Arc;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use crate::state::AppState;
use crate::business_sandbox::state::*;

pub async fn game_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_game_ws(socket, state, game_id))
}

async fn handle_game_ws(
    socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
    game_id: String,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(256);

    // 注册到游戏管理器
    state.business_sandbox.register_ws(&game_id, tx).await;

    // 创建游戏
    if let Err(e) = state.business_sandbox.manager.create_game(&game_id).await {
        let _ = sender.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&GameEvent::GameOver {
                reason: format!("创建游戏失败: {}", e),
            }).unwrap()
        )).await;
        return;
    }

    // 发送初始状态
    if let Ok(initial_state) = state.business_sandbox.manager.get_state(&game_id).await {
        let msg = serde_json::to_string(&GameEvent::StateUpdate(Box::new(initial_state))).unwrap();
        let _ = sender.send(axum::extract::ws::Message::Text(msg)).await;
    }

    // 发送要求玩家输入竞标策略
    let ask_msg = serde_json::to_string(&GameEvent::AskDecision {
        year: 1, quarter: 0,
        decision_type: "bidding".into(),
    }).unwrap();
    let _ = sender.send(axum::extract::ws::Message::Text(ask_msg)).await;

    // 接收玩家消息
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 处理玩家输入
    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            if let Ok(action) = serde_json::from_str::<PlayerAction>(&text) {
                match state.business_sandbox.manager.handle_action(&game_id, action).await {
                    Ok(events) => {
                        for event in events {
                            let json = serde_json::to_string(&event).unwrap();
                            let _ = state.business_sandbox.broadcast(&game_id, &json).await;
                        }
                    }
                    Err(e) => {
                        let err_msg = serde_json::to_string(
                            &GameEvent::Message(format!("错误: {}", e))
                        ).unwrap();
                        let _ = state.business_sandbox.broadcast(&game_id, &err_msg).await;
                    }
                }
            }
        }
    }

    send_task.abort();
}
```

- [ ] **Step 2: 修改 state.rs 添加 BusinessSandboxManager**

Edit `rust/api/src/state.rs` - add to `AppState`:

```rust
// Add import
use crate::business_sandbox::engine::GameManager;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct BusinessSandboxState {
    pub manager: GameManager,
    ws_senders: tokio::sync::Mutex<HashMap<String, Vec<mpsc::Sender<String>>>>,
}

impl BusinessSandboxState {
    pub fn new() -> Self {
        Self {
            manager: GameManager::new(),
            ws_senders: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub async fn register_ws(&self, game_id: &str, tx: mpsc::Sender<String>) {
        let mut senders = self.ws_senders.lock().await;
        senders.entry(game_id.to_string())
            .or_default()
            .push(tx);
    }

    pub async fn broadcast(&self, game_id: &str, message: &str) {
        let mut senders = self.ws_senders.lock().await;
        if let Some(entries) = senders.get_mut(game_id) {
            entries.retain(|tx| tx.try_send(message.to_string()).is_ok());
        }
    }
}
```

Add to `AppState` struct:

```rust
pub struct AppState {
    // ... existing fields ...
    pub business_sandbox: BusinessSandboxState,
}
```

Update `AppState::new()` or wherever it's constructed.

- [ ] **Step 3: 在 main.rs 挂载路由**

Edit `rust/api/src/main.rs`:

```rust
// Add mod declaration
mod business_sandbox;

// In build_router(), add WS route:
.route("/ws/game/:game_id", axum::routing::get(crate::business_sandbox::ws_handler::game_ws_handler))
```

- [ ] **Step 4: Rust compilation check**

```bash
cd rust && cargo check --package api 2>&1 | head -40
```

Fix any compilation errors.

- [ ] **Step 5: Commit**

```bash
git add rust/api/src/ rust/api/src/business_sandbox/ws_handler.rs
git commit -m "feat(business-sandbox): 实现WebSocket路由和事件广播"
```

---

### Task 7: AI 竞争对手

**File:** `rust/api/src/business_sandbox/ai_competitor.rs`

- [ ] **Step 1: 实现 AI 竞争对手调用**

```rust
use std::sync::Arc;
use crate::business_sandbox::state::*;
use ai_gateway::{GatewayRegistry, PromptBuilder, CallConfig, DynamicContext};

/// AI 竞争对手的决策输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorDecision {
    pub company_name: String,
    pub marketing_spend: u32,
    pub product_focus: Product,
    pub pricing_strategy: String, // "aggressive" | "normal" | "premium"
}

/// 调用 AI 竞争对手生成年度决策
pub async fn generate_competitor_decisions(
    gateway: Arc<GatewayRegistry>,
    registry: Arc<registry::SoulRegistry>,
    state: &BusinessGameState,
    market_data: &str,
) -> Vec<CompetitorDecision> {
    let competitor_souls = vec!["CEO", "CFO", "CMO", "COO", "管理咨询顾问"];

    let mut decisions = Vec::new();
    for soul_name in &competitor_souls {
        if let Some(profile) = registry.get_soul(soul_name) {
            let prompt = format!(
                "你是一家与玩家竞争的公司的{soul_role}。当前市场情况：{market_data}\n\
                 请做出本年度的经营决策：\n\
                 1. 营销投入（M）\n\
                 2. 主打产品\n\
                 3. 定价策略（激进/正常/高溢价）\n\
                 请以JSON格式输出决策。",
                soul_role = soul_name,
                market_data = market_data,
            );

            if let Some(provider) = gateway.pick_provider(None) {
                let call_config = CallConfig {
                    temperature: Some(0.7),
                    max_tokens: Some(500),
                    stream: false,
                    model: profile.model.clone(),
                    ..Default::default()
                };

                let prompt_msg = ai_gateway::Prompt {
                    system: Some(profile.summon_prompt.clone()),
                    messages: vec![
                        ("user".into(), prompt),
                    ],
                };

                let mut rx = gateway.call(&prompt_msg, &call_config);
                let mut full_response = String::new();
                while let Some(Ok(chunk)) = rx.recv().await {
                    full_response.push_str(&chunk.content);
                }

                // 尝试解析 JSON 决策
                if let Ok(decision) = serde_json::from_str::<CompetitorDecision>(&full_response) {
                    decisions.push(decision);
                } else {
                    // 解析失败时用默认值
                    decisions.push(CompetitorDecision {
                        company_name: soul_name.to_string(),
                        marketing_spend: 2,
                        product_focus: Product::BenMa,
                        pricing_strategy: "normal".into(),
                    });
                }
            }
        }
    }

    decisions
}
```

- [ ] **Step 2: Commit**

```bash
git add rust/api/src/business_sandbox/ai_competitor.rs
git commit -m "feat(business-sandbox): 实现AI竞争对手调用"
```

---

### Task 8: 创建 Godot 2D 前端项目

**Directory:** `godot/` (新项目根目录)

- [ ] **Step 1: 初始化 Godot 项目**

```bash
mkdir -p godot/scenes godot/scripts/popups godot/ui godot/tests
```

Create `godot/project.godot`:
```gdscript
[application]
config/name="创业沙盘"
config/description="商业全价值链经营模拟器"
run/main_scene="res://scenes/MainGame.tscn"
config/icon="res://ui/icon.png"

[rendering]
renderer/backend=opengl3  ; Web兼容
```

- [ ] **Step 2: 创建 WebSocketManager 单例**

Write `godot/scripts/websocket_manager.gd`:
```gdscript
extends Node

var socket = WebSocketPeer.new()
var connected = false
var server_url = "ws://127.0.0.1:3097/ws/game/"
var game_id = ""
signal connected()
signal disconnected()
signal message_received(data: Dictionary)
signal connection_error()

func _ready():
    socket.connect("connection_established", Callable(self, "_on_connected"))
    socket.connect("connection_closed", Callable(self, "_on_disconnected"))
    socket.connect("data_received", Callable(self, "_on_data"))
    socket.connect("connection_error", Callable(self, "_on_error"))

func connect_to_server(g_id: String):
    game_id = g_id
    var url = server_url + g_id
    socket.connect_to_url(url)
    
func send_action(action: Dictionary):
    if connected:
        socket.send(JSON.stringify(action))

func _process(delta):
    if socket.is_connected():
        socket.poll()
        
func _on_connected():
    connected = true
    emit_signal("connected")
    
func _on_disconnected():
    connected = false
    emit_signal("disconnected")
    
func _on_data():
    var raw = socket.get_packet()
    var json_str = raw.get_string_from_utf8()
    var data = JSON.parse_string(json_str)
    if data is Dictionary:
        emit_signal("message_received", data)

func _on_error():
    emit_signal("connection_error")
```

- [ ] **Step 3: 创建 MainGame 主场景**

Write `godot/scripts/main_game.gd`:
```gdscript
extends Control

@onready var top_bar = $TopBar
@onready var production_area = $ProductionArea
@onready var bottom_panel = $BottomPanel
@onready var ws_manager = $WebSocketManager

var game_state: Dictionary = {}

func _ready():
    ws_manager.connect("message_received", Callable(self, "_on_ws_message"))
    ws_manager.connect_to_server("game_001")

func _on_ws_message(data: Dictionary):
    match data.get("event"):
        "state_update":
            game_state = data.get("data", {})
            update_ui()
        "ask_decision":
            show_decision_prompt(data.get("decision_type", ""))
        "message":
            show_message(data.get("data", ""))
        "game_over":
            show_game_over(data.get("data", {}).get("reason", ""))
        "annual_report":
            show_annual_report(data.get("data", {}))

func update_ui():
    # 更新顶部状态栏
    top_bar.update(
        game_state.get("game_year", 1),
        game_state.get("game_quarter", 1),
        game_state.get("cash", 0)
    )
    # 更新生产线视窗
    production_area.update(game_state.get("factories", []))
    # 更新底部面板
    bottom_panel.update(game_state)

func show_decision_prompt(decision_type: String):
    match decision_type:
        "bidding":
            open_bidding_dialog()
        "next_quarter":
            # 显示"执行下一季度"按钮
            bottom_panel.show_next_quarter_button()
        
func open_bidding_dialog():
    # 打开竞标策略弹窗
    pass

func show_message(text: String):
    # 显示系统消息
    pass

func show_game_over(reason: String):
    # 显示游戏结束弹窗
    pass

func show_annual_report(report: Dictionary):
    # 显示年度报告弹窗
    pass
```

- [ ] **Step 4: 创建场景文件（空模板）**

Create basic `MainGame.tscn` structure via Godot editor (or write .tscn by hand). For now, the plan is to open this in Godot editor and set up the UI layout.

**关键 UI 节点：**
```
MainGame (Control, Stretch全屏)
├── TopBar (HBoxContainer, 锚定顶部)
│   ├── YearLabel (Label)
│   ├── QuarterLabel (Label)
│   └── CashLabel (Label + 金币图标)
├── ProductionArea (VBoxContainer, 锚定中间)
│   ├── Line1 (ProductionLine 自定义节点)
│   ├── Line2
│   ├── Line3
│   └── Line4
├── BottomPanel (HBoxContainer, 锚定底部)
│   ├── LoanDisplay (财务数据)
│   ├── NextQuarterBtn (Button)
│   ├── AnnualReportBtn (Button)
│   └── DiscountBtn (Button)
└── PopupLayer (Control, 全屏覆盖)
    ├── BiddingDialog (Panel)
    ├── AnnualReportDialog (Panel)
    ├── DiscountConfirm (Panel)
    └── GameOverScreen (Panel)
```

- [ ] **Step 5: Commit**

```bash
git add godot/
git commit -m "feat(godot): 初始化创业沙盘Godot 2D项目结构和WS通信"
```

---

## 未来任务（阶段4 — 动画和发布）

- 使用 `Tween` 动画实现原料→产线→成品流动
- 替换占位色块为实际 UI 贴图
- Web (HTML5) + Desktop 双端导出配置
- 导出测试
