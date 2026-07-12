use serde::{Deserialize, Serialize};

// ── Product ──

/// 四种产品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Product {
    /// 奔马 — 基础产品
    BenMa,
    /// 猛虎 — 升级产品
    MengHu,
    /// 飞鹰 — 中级产品
    FeiYing,
    /// 天龙 — 高级产品
    TianLong,
}

impl Product {
    /// 原材料成本（每单位）
    pub fn raw_material_cost(&self) -> u32 {
        match self {
            Product::BenMa => 1,
            Product::MengHu => 2,
            Product::FeiYing => 3,
            Product::TianLong => 4,
        }
    }

    /// 生产成本（每单位）
    pub fn production_cost(&self) -> u32 {
        match self {
            Product::BenMa | Product::MengHu => 1,
            Product::FeiYing | Product::TianLong => 2,
        }
    }

    /// 中文名称
    pub fn name_cn(&self) -> &'static str {
        match self {
            Product::BenMa => "奔马",
            Product::MengHu => "猛虎",
            Product::FeiYing => "飞鹰",
            Product::TianLong => "天龙",
        }
    }
}

// ── LineType ──

/// 产线类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineType {
    /// 手工线
    Manual,
    /// 半自动线
    SemiAuto,
    /// 全自动线
    FullAuto,
}

impl LineType {
    /// 建设时间（季度）
    pub fn build_time(&self) -> u32 {
        match self {
            LineType::Manual => 1,
            LineType::SemiAuto => 4,
            LineType::FullAuto => 4,
        }
    }

    /// 每季度建设成本
    pub fn build_cost_per_quarter(&self) -> u32 {
        match self {
            LineType::Manual => 4,
            LineType::SemiAuto => 2,
            LineType::FullAuto => 4,
        }
    }

    /// 总建设成本
    pub fn total_build_cost(&self) -> u32 {
        match self {
            LineType::Manual => 4,
            LineType::SemiAuto => 8,
            LineType::FullAuto => 16,
        }
    }

    /// 转产时间（季度）
    pub fn switch_time(&self) -> u32 {
        match self {
            LineType::Manual => 0,
            LineType::SemiAuto => 1,
            LineType::FullAuto => 2,
        }
    }

    /// 转产成本
    pub fn switch_cost(&self) -> u32 {
        match self {
            LineType::Manual => 0,
            LineType::SemiAuto => 1,
            LineType::FullAuto => 4,
        }
    }

    /// 残值
    pub fn salvage_value(&self) -> u32 {
        match self {
            LineType::Manual => 2,
            LineType::SemiAuto => 3,
            LineType::FullAuto => 6,
        }
    }
}

// ── LineStatus ──

/// 产线状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStatus {
    /// 空闲
    Idle,
    /// 生产中（含产品类型）
    Producing(Product),
    /// 建设中（含剩余季度数）
    Building(u32),
    /// 转产中（含剩余季度数和目标产品）
    SwitchingTo(u32, Product),
}

// ── Data Structures ──

/// 生产线的运行时表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionLine {
    pub id: u32,
    pub line_type: LineType,
    pub status: LineStatus,
    /// 当前设定的产品（None 表示未设定）
    pub product: Option<Product>,
}

/// 工厂
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factory {
    pub id: String,
    pub name: String,
    /// 最大产线数量
    pub capacity: u32,
    /// 工厂总价值
    pub value: u32,
    /// 产线列表
    pub lines: Vec<ProductionLine>,
}

/// 产品研发进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRDA {
    pub product: Product,
    /// 当前进度
    pub progress: u32,
    /// 研发所需总量
    pub total: u32,
    /// 是否已完成
    pub completed: bool,
}

/// 应收账款
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountReceivable {
    pub amount: u32,
    /// 剩余到期季度数
    pub due_quarters: u32,
}

/// 贷款
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub amount: u32,
    /// 剩余还款季度数
    pub remaining_quarters: u32,
    /// 年利率（百分比，如 5.0 表示 5%）
    pub annual_interest_rate: f64,
}

/// 长期贷款额度槽位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermLoanSlot {
    pub year: u32,
    pub amount: u32,
    /// 是否已启用（已放贷）
    pub active: bool,
}

/// 市场
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub name: String,
    /// 市场开发度
    pub developed: bool,
    /// 市场排名
    pub rank: u32,
    /// 上年度销售额
    pub last_year_sales: u32,
}

/// 订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub product: Product,
    pub quantity: u32,
    /// 单价
    pub unit_price: u32,
    /// 账期（季度数）
    pub account_period: u32,
    /// 是否已交付
    pub delivered: bool,
    /// 是否加急
    pub urgent: bool,
}

/// 竞标策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiddingStrategy {
    pub market_name: String,
    /// 市场营销投入
    pub marketing_spend: u32,
}

/// 在制品（WIP）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WIPItem {
    pub line_id: u32,
    pub product: Product,
    /// 生产进度（0-100%）
    pub progress: u32,
}

/// 成品库存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedGoods {
    pub product: Product,
    pub quantity: u32,
}

/// 决策项（key-value 形式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionItem {
    pub key: String,
    pub value: serde_json::Value,
}

// ── BusinessGameState ──

/// 创业沙盘完整游戏状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessGameState {
    // ── 时间 ──
    pub game_year: u32,
    pub game_quarter: u32,  // 1-4

    // ── 财务 ──
    pub cash: u32,

    // ── 资产 ──
    pub factories: Vec<Factory>,
    pub products_rd: Vec<ProductRDA>,

    // ── 原材料 ──
    pub raw_material_orders: u32,
    pub raw_material_inventory: u32,

    // ── 生产与库存 ──
    pub work_in_progress: Vec<WIPItem>,
    pub finished_goods: Vec<FinishedGoods>,

    // ── 应收与贷款 ──
    pub accounts_receivable: Vec<AccountReceivable>,
    pub long_term_loans: Vec<LongTermLoanSlot>,
    pub short_term_loans: Vec<Loan>,

    // ── 市场与订单 ──
    pub markets: Vec<Market>,
    pub orders: Vec<Order>,
    pub bidding_strategies: Vec<BiddingStrategy>,

    // ── 当前阶段 ──
    pub phase: u32,  // 1-4

    // ── 状态标志 ──
    pub game_over: bool,
    pub game_over_reason: Option<String>,
}

// ── GameEvent ──

/// WebSocket 事件（服务端 -> 客户端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum GameEvent {
    /// 全量状态更新
    StateUpdate(Box<BusinessGameState>),
    /// 阶段变更
    PhaseChange {
        year: u32,
        quarter: u32,
        phase: String,
    },
    /// 请求玩家决策
    AskDecision {
        year: u32,
        quarter: u32,
        decision_type: String,
    },
    /// 年度财务报告
    AnnualReport(Box<AnnualReport>),
    /// 选单会
    OrderMeeting {
        market: String,
        available_orders: Vec<Order>,
    },
    /// 游戏结束
    GameOver {
        reason: String,
    },
    /// 通用消息
    Message(String),
}

// ── PlayerAction ──

/// 客户端操作（客户端 -> 服务端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PlayerAction {
    /// 开始游戏
    StartGame,
    /// 提交竞标策略
    SubmitBidding {
        strategies: Vec<BiddingStrategy>,
    },
    /// 选择订单
    SelectOrder {
        order_ids: Vec<String>,
    },
    /// 做出决策
    MakeDecision {
        decisions: Vec<DecisionItem>,
    },
    /// 进入下一季度
    NextQuarter,
    /// 贴现应收账款
    DiscountReceivable {
        amount: u32,
    },
    /// 申请贷款
    TakeLoan {
        loan_type: String,
        amount: u32,
    },
}

// ── Financial Reports ──

/// 年度财务报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualReport {
    pub year: u32,
    pub income_statement: IncomeStatement,
    pub balance_sheet: BalanceSheet,
    pub expense_sheet: ExpenseSheet,
}

/// 利润表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    /// 营业收入
    pub revenue: u32,
    /// 营业成本
    pub cost_of_goods_sold: u32,
    /// 毛利润
    pub gross_profit: i32,
    /// 销售费用
    pub sales_expense: u32,
    /// 管理费用
    pub admin_expense: u32,
    /// 研发费用
    pub rd_expense: u32,
    /// 折旧
    pub depreciation: u32,
    /// 营业利润
    pub operating_profit: i32,
    /// 利息支出
    pub interest_expense: u32,
    /// 贴现费用
    pub discount_fee: u32,
    /// 税金
    pub tax: u32,
    /// 净利润
    pub net_profit: i32,
}

/// 资产负债表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheet {
    // ── 流动资产 ──
    /// 现金
    pub cash: u32,
    /// 应收账款
    pub accounts_receivable: u32,
    /// 原材料
    pub raw_material: u32,
    /// 在制品
    pub work_in_process: u32,
    /// 产成品
    pub finished_goods: u32,
    /// 流动资产合计
    pub total_current_assets: u32,
    // ── 固定资产 ──
    /// 工厂
    pub factory: u32,
    /// 生产线
    pub production_lines: u32,
    /// 固定资产合计
    pub total_fixed_assets: u32,
    /// 资产总计
    pub total_assets: u32,
    // ── 负债 ──
    /// 长期贷款
    pub long_term_loans: u32,
    /// 短期贷款
    pub short_term_loans: u32,
    /// 负债合计
    pub total_liabilities: u32,
    // ── 所有者权益 ──
    /// 所有者权益
    pub equity: i32,
}

/// 费用明细表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseSheet {
    /// 新市场开拓投资
    pub new_market_investment: u32,
    /// 产品研发投资
    pub product_rd_investment: u32,
    /// 季度行政管理费
    pub quarterly_admin_fees: u32,
    /// 厂房租金
    pub factory_rent: u32,
    /// 设备维护费
    pub line_maintenance: u32,
    /// 产品转换费
    pub product_switch_fees: u32,
    /// 销售费用
    pub sales_expenses: u32,
    /// 折旧
    pub depreciation: u32,
    /// 利息与贴现费用
    pub interest_and_discount: u32,
    /// 税金
    pub tax: u32,
}
