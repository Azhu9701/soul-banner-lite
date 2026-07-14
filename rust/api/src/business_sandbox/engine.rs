// ── 创业沙盘游戏核心引擎 ──

use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::business_sandbox::state::*;
use crate::business_sandbox::errors::GameError;
use crate::business_sandbox::finance::*;
use crate::business_sandbox::production::*;

// ── GameManager ──

/// 游戏实例管理器（线程安全）
pub struct GameManager {
    games: Mutex<HashMap<String, GameInstance>>,
}

/// 游戏运行时实例
pub struct GameInstance {
    pub state: BusinessGameState,
    pub annual_reports: Vec<AnnualReport>,
    /// "waiting" | "year_start" | "quarter_ops" | "year_end"
    pub current_phase: String,
    // ── 原材料订购 ──
    pub raw_material_order_quantity: u32,
    // ── 年度财务跟踪（每年清零） ──
    pub annual_revenue: u32,
    pub annual_cost_of_goods: u32,
    pub annual_rd_expense: u32,
    pub annual_sales_expense: u32,
    pub annual_interest_expense: u32,
    pub annual_discount_fee: u32,
    pub annual_admin_expense: u32,
    // ── 订单会暂存 ──
    pub pending_orders: Vec<Order>,
}

impl GameManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            games: Mutex::new(HashMap::new()),
        }
    }

    /// 创建新游戏
    pub async fn create_game(&self, game_id: &str) -> Result<(), GameError> {
        let mut games = self.games.lock().await;
        if games.contains_key(game_id) {
            return Err(GameError::Internal(format!("游戏 {} 已存在", game_id)));
        }
        games.insert(
            game_id.to_string(),
            GameInstance {
                state: Self::initial_state(),
                annual_reports: Vec::new(),
                current_phase: "waiting".to_string(),
                raw_material_order_quantity: 2,
                annual_revenue: 0,
                annual_cost_of_goods: 0,
                annual_rd_expense: 0,
                annual_sales_expense: 0,
                annual_interest_expense: 0,
                annual_discount_fee: 0,
                annual_admin_expense: 0,
                pending_orders: Vec::new(),
            },
        );
        Ok(())
    }

    /// 返回 PDF 规则规定的初始状态
    pub fn initial_state() -> BusinessGameState {
        BusinessGameState {
            game_year: 1,
            game_quarter: 1,
            // 初始现金 12M
            cash: 12,
            // 1 个老工厂，容量 4，价值 20M
            factories: vec![Factory {
                id: "F1".to_string(),
                name: "老工厂".to_string(),
                capacity: 4,
                value: 20,
                // 含 1 条手工线生产奔马
                lines: vec![ProductionLine {
                    id: 1,
                    line_type: LineType::Manual,
                    status: LineStatus::Producing(Product::BenMa),
                    product: Some(Product::BenMa),
                }],
            }],
            // 产品研发（仅奔马已研发完成，其他产品需玩家决定是否投入研发）
            products_rd: vec![
                ProductRDA {
                    product: Product::BenMa,
                    progress: 1,
                    total: 1,
                    completed: true,
                },
            ],
            // 原材料库存 8 个单位，采购在途 2 个
            raw_material_orders: 2,
            raw_material_inventory: 8,
            // 在制品 1 条（产线1，奔马，进度1）
            work_in_progress: vec![WIPItem {
                line_id: 1,
                product: Product::BenMa,
                progress: 1,
            }],
            // 成品库 4 批奔马
            finished_goods: vec![FinishedGoods {
                product: Product::BenMa,
                quantity: 4,
            }],
            // 应收账款 12M，账期 4 季度
            accounts_receivable: vec![AccountReceivable {
                amount: 12,
                due_quarters: 4,
            }],
            long_term_loans: Vec::new(),
            short_term_loans: Vec::new(),
            // 平城市场（已开发，排名第 1），其余未开发
            markets: vec![
                Market {
                    name: "平城".to_string(),
                    developed: true,
                    rank: 1,
                    last_year_sales: 0,
                },
                Market {
                    name: "南城".to_string(),
                    developed: false,
                    rank: 0,
                    last_year_sales: 0,
                },
                Market {
                    name: "北城".to_string(),
                    developed: false,
                    rank: 0,
                    last_year_sales: 0,
                },
                Market {
                    name: "东城".to_string(),
                    developed: false,
                    rank: 0,
                    last_year_sales: 0,
                },
                Market {
                    name: "西城".to_string(),
                    developed: false,
                    rank: 0,
                    last_year_sales: 0,
                },
            ],
            orders: Vec::new(),
            bidding_strategies: Vec::new(),
            phase: 1,
            game_over: false,
            game_over_reason: None,
        }
    }

    /// 获取游戏状态
    pub async fn get_state(&self, game_id: &str) -> Result<BusinessGameState, GameError> {
        let games = self.games.lock().await;
        games
            .get(game_id)
            .map(|g| g.state.clone())
            .ok_or_else(|| GameError::GameNotFound(game_id.to_string()))
    }

    /// 处理玩家操作，返回事件列表
    pub async fn handle_action(
        &self,
        game_id: &str,
        action: PlayerAction,
    ) -> Result<Vec<GameEvent>, GameError> {
        let mut games = self.games.lock().await;
        let instance = games
            .get_mut(game_id)
            .ok_or_else(|| GameError::GameNotFound(game_id.to_string()))?;

        if instance.state.game_over {
            return Err(GameError::GameAlreadyOver);
        }

        match action {
            PlayerAction::StartGame => {
                // 初始化状态，返回全量状态 + 请求竞标决策
                instance.current_phase = "year_start".to_string();
                let state = instance.state.clone();
                Ok(vec![
                    GameEvent::StateUpdate { data: Box::new(state) },
                    GameEvent::AskDecision { data: AskDecisionData { year: instance.state.game_year, quarter: instance.state.game_quarter, decision_type: "bidding".to_string() } },
                ])
            }

            PlayerAction::SubmitBidding { strategies } => {
                // 扣除营销费
                let total_spend: u32 = strategies.iter().map(|s| s.marketing_spend).sum();
                if instance.state.cash < total_spend {
                    return Err(GameError::InsufficientFunds {
                        required: total_spend as f64,
                        available: instance.state.cash as f64,
                    });
                }
                instance.state.cash -= total_spend;
                instance.state.bidding_strategies = strategies;

                // 会计年度初始化（每年 Q1 重置）
                if instance.state.game_quarter == 1 {
                    instance.annual_revenue = 0;
                    instance.annual_cost_of_goods = 0;
                    instance.annual_rd_expense = 0;
                    instance.annual_sales_expense = 0;
                    instance.annual_interest_expense = 0;
                    instance.annual_discount_fee = 0;
                    instance.annual_admin_expense = 0;
                }
                instance.annual_sales_expense += total_spend;
                instance.current_phase = "quarter_ops".to_string();

                // 为每个已开发市场生成订单
                for market in &instance.state.markets {
                    if !market.developed {
                        continue;
                    }
                    let market_spend = instance
                        .state
                        .bidding_strategies
                        .iter()
                        .find(|s| s.market_name == market.name)
                        .map(|s| s.marketing_spend)
                        .unwrap_or(0);
                    let orders = generate_orders(
                        market_spend,
                        market,
                        &instance.state,
                    );
                    instance.pending_orders.extend(orders);
                }

                let mut events: Vec<GameEvent> = vec![
                    GameEvent::Message { data: format!("提交竞标策略成功，营销费用 {}M", total_spend) },
                    GameEvent::PhaseChange { data: PhaseChangeData { year: instance.state.game_year, quarter: instance.state.game_quarter, phase: "quarter_ops".to_string() } },
                ];

                // 发送订单会事件
                if !instance.pending_orders.is_empty() {
                    // 使用第一个已开发市场作为订单会市场
                    let developed_market = instance
                        .state
                        .markets
                        .iter()
                        .find(|m| m.developed)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "平城".to_string());
                    let available = instance.pending_orders.clone();
                    events.push(GameEvent::OrderMeeting {
                        data: OrderMeetingData {
                            market: developed_market,
                            available_orders: available,
                        },
                    });
                }

                let state = instance.state.clone();
                events.push(GameEvent::StateUpdate { data: Box::new(state) });

                Ok(events)
            }

            PlayerAction::NextQuarter => {
                let quarter_events = Self::execute_quarter(instance)?;
                let mut events = quarter_events;

                if instance.state.game_quarter == 4 {
                    // Q4 还执行年末结算
                    let year_end_events = Self::execute_year_end(instance)?;
                    events.extend(year_end_events);

                    // 重置年度跟踪
                    instance.annual_revenue = 0;
                    instance.annual_cost_of_goods = 0;
                    instance.annual_rd_expense = 0;
                    instance.annual_sales_expense = 0;
                    instance.annual_interest_expense = 0;
                    instance.annual_discount_fee = 0;
                    instance.annual_admin_expense = 0;

                    // 进入下一年
                    instance.state.game_year += 1;
                    instance.state.game_quarter = 1;
                    instance.current_phase = "year_start".to_string();
                } else {
                    instance.state.game_quarter += 1;
                    instance.current_phase = "quarter_ops".to_string();
                }

                let state = instance.state.clone();
                events.push(GameEvent::StateUpdate { data: Box::new(state) });

                Ok(events)
            }

            PlayerAction::DiscountReceivable { amount } => {
                // 贴现，扣除手续费
                let fee = calc_discount_fee(amount);
                let net_amount = amount.saturating_sub(fee);

                // 从应收账款中扣除对应金额
                let mut remaining = amount;
                let mut new_receivables = Vec::new();
                for ar in instance.state.accounts_receivable.drain(..) {
                    if remaining > 0 {
                        let take = ar.amount.min(remaining);
                        remaining -= take;
                        if take < ar.amount {
                            new_receivables.push(AccountReceivable {
                                amount: ar.amount - take,
                                due_quarters: ar.due_quarters,
                            });
                        }
                    } else {
                        new_receivables.push(ar);
                    }
                }

                if remaining > 0 {
                    return Err(GameError::InvalidAction(format!(
                        "应收账款不足，需要贴现 {}M",
                        amount
                    )));
                }

                instance.state.accounts_receivable = new_receivables;
                instance.state.cash += net_amount;
                instance.annual_discount_fee += fee;

                Ok(vec![GameEvent::Message { data: format!(
                    "贴现成功，获得 {}M，手续费 {}M",
                    net_amount, fee
                ) }])
            }

            PlayerAction::TakeLoan { loan_type, amount } => {
                validate_loan_amount(amount)
                    .map_err(|e| GameError::InvalidAction(e))?;

                match loan_type.as_str() {
                    "short_term" | "short" => {
                        let equity = calculate_equity(&instance.state);
                        let (_, short_limit) = calc_loan_limit(equity);
                        let current_short: u32 =
                            instance.state.short_term_loans.iter().map(|l| l.amount).sum();
                        if current_short + amount > short_limit {
                            return Err(GameError::InvalidAction(format!(
                                "短期贷款额度不足：已用 {}M，限额 {}M",
                                current_short, short_limit
                            )));
                        }
                        instance.state.short_term_loans.push(Loan {
                            amount,
                            remaining_quarters: 4,
                            annual_interest_rate: 10.0,
                        });
                        instance.state.cash += amount;
                        Ok(vec![GameEvent::Message { data: format!(
                            "短期贷款成功 {}M",
                            amount
                        ) }])
                    }
                    "long_term" | "long" => {
                        let equity = calculate_equity(&instance.state);
                        let (long_limit, _) = calc_loan_limit(equity);
                        let current_long: u32 = instance
                            .state
                            .long_term_loans
                            .iter()
                            .filter(|s| s.active)
                            .map(|s| s.amount)
                            .sum();
                        if current_long + amount > long_limit {
                            return Err(GameError::InvalidAction(format!(
                                "长期贷款额度不足：已用 {}M，限额 {}M",
                                current_long, long_limit
                            )));
                        }
                        instance.state.long_term_loans.push(LongTermLoanSlot {
                            year: instance.state.game_year,
                            amount,
                            active: true,
                        });
                        instance.state.cash += amount;
                        Ok(vec![GameEvent::Message { data: format!(
                            "长期贷款成功 {}M",
                            amount
                        ) }])
                    }
                    _ => Err(GameError::InvalidAction(format!(
                        "未知贷款类型: {}",
                        loan_type
                    ))),
                }
            }

            PlayerAction::SelectOrder { order_ids } | PlayerAction::ConfirmOrders { order_ids } => {
                if instance.pending_orders.is_empty() {
                    return Err(GameError::InvalidAction("没有待选订单".to_string()));
                }
                let mut selected = Vec::new();
                let mut remaining = Vec::new();
                for order in instance.pending_orders.drain(..) {
                    if order_ids.contains(&order.id) {
                        selected.push(order);
                    } else {
                        remaining.push(order);
                    }
                }
                instance.pending_orders = remaining;
                // 将选中的订单加入 game_state.orders
                instance.state.orders.extend(selected.clone());
                Ok(vec![GameEvent::Message {
                    data: format!("已确认 {} 张订单", selected.len()),
                }])
            }

            PlayerAction::MakeDecision { decisions } => {
                let mut events = Vec::new();
                for item in &decisions {
                    match item.key.as_str() {
                        "raw_material_order" => {
                            if let Some(qty) = item.value.as_u64() {
                                instance.raw_material_order_quantity = qty as u32;
                                events.push(GameEvent::Message {
                                    data: format!("原材料订购量设置为 {} 单位", qty),
                                });
                            }
                        }
                        "market_development" => {
                            if let Some(market_name) = item.value.as_str() {
                                if let Some(market) = instance
                                    .state
                                    .markets
                                    .iter_mut()
                                    .find(|m| m.name == market_name)
                                {
                                    if !market.developed {
                                        let dev_cost = 2u32; // 市场开发费 2M
                                        if instance.state.cash < dev_cost {
                                            return Err(GameError::InsufficientFunds {
                                                required: dev_cost as f64,
                                                available: instance.state.cash as f64,
                                            });
                                        }
                                        instance.state.cash -= dev_cost;
                                        market.developed = true;
                                        events.push(GameEvent::Message {
                                            data: format!("市场 {} 开发成功，费用 {}M", market_name, dev_cost),
                                        });
                                    }
                                }
                            }
                        }
                        _ => {
                            events.push(GameEvent::Message {
                                data: format!("未知决策项: {}", item.key),
                            });
                        }
                    }
                }
                let state = instance.state.clone();
                events.push(GameEvent::StateUpdate { data: Box::new(state) });
                Ok(events)
            }

            PlayerAction::StartRDA { product } => {
                let product = parse_product(&product)
                    .ok_or_else(|| GameError::InvalidAction(format!("未知产品: {}", product)))?;
                if product == Product::BenMa {
                    return Err(GameError::InvalidAction("奔马已拥有，无需研发".to_string()));
                }
                // 检查是否已开始或已完成
                if let Some(rd) = instance.state.products_rd.iter().find(|r| r.product == product) {
                    if rd.completed {
                        return Err(GameError::InvalidAction(format!("{:?} 已研发完成", product)));
                    }
                    return Err(GameError::InvalidAction(format!("{:?} 已在研发中", product)));
                }
                let has_feijing = instance
                    .state
                    .products_rd
                    .iter()
                    .any(|r| r.product == Product::FeiYing && r.completed);
                let total = product_rd_total_quarters(product, has_feijing);
                instance.state.products_rd.push(ProductRDA {
                    product,
                    progress: 0,
                    total,
                    completed: false,
                });
                Ok(vec![GameEvent::Message {
                    data: format!("开始研发 {:?}，预计 {} 季度完成", product, total),
                }])
            }

            PlayerAction::BuildLine { line_type, factory_id } => {
                let lt = parse_line_type(&line_type)
                    .ok_or_else(|| GameError::InvalidAction(format!("未知产线类型: {}", line_type)))?;
                let build_cost = lt.build_cost_per_quarter();
                if instance.state.cash < build_cost {
                    return Err(GameError::InsufficientFunds {
                        required: build_cost as f64,
                        available: instance.state.cash as f64,
                    });
                }
                let factory = instance
                    .state
                    .factories
                    .iter_mut()
                    .find(|f| f.id == factory_id)
                    .ok_or_else(|| GameError::InvalidAction(format!("工厂不存在: {}", factory_id)))?;
                if factory.lines.len() as u32 >= factory.capacity {
                    return Err(GameError::InvalidAction(format!(
                        "工厂 {} 容量已满（{} 条）",
                        factory_id, factory.capacity
                    )));
                }
                let new_id = factory.lines.iter().map(|l| l.id).max().unwrap_or(0) + 1;
                instance.state.cash -= build_cost;
                factory.lines.push(ProductionLine {
                    id: new_id,
                    line_type: lt,
                    status: LineStatus::Building(lt.build_time()),
                    product: None,
                });
                Ok(vec![GameEvent::Message {
                    data: format!(
                        "开始建设产线{}（{:?}），首期费用 {}M，预计 {} 季度完成",
                        new_id, lt, build_cost, lt.build_time()
                    ),
                }])
            }

            PlayerAction::SwitchProduct { line_id, product } => {
                let product = parse_product(&product)
                    .ok_or_else(|| GameError::InvalidAction(format!("未知产品: {}", product)))?;
                // 检查研发是否完成
                let rd_completed = instance
                    .state
                    .products_rd
                    .iter()
                    .any(|r| r.product == product && r.completed);
                if product != Product::BenMa && !rd_completed {
                    return Err(GameError::RDAIncomplete(format!(
                        "{:?} 尚未研发完成",
                        product
                    )));
                }
                let line = instance
                    .state
                    .factories
                    .iter_mut()
                    .flat_map(|f| &mut f.lines)
                    .find(|l| l.id == line_id)
                    .ok_or_else(|| GameError::InvalidAction(format!("产线不存在: {}", line_id)))?;
                if !matches!(line.status, LineStatus::Idle) {
                    return Err(GameError::InvalidAction(format!(
                        "产线{} 非空闲状态，无法转产",
                        line_id
                    )));
                }
                let switch_cost = line.line_type.switch_cost();
                let switch_time = line.line_type.switch_time();
                if switch_time == 0 {
                    // 手工线即时转产
                    line.product = Some(product);
                    Ok(vec![GameEvent::Message {
                        data: format!("产线{} 即时转产为 {:?}", line_id, product),
                    }])
                } else {
                    if instance.state.cash < switch_cost {
                        return Err(GameError::InsufficientFunds {
                            required: switch_cost as f64,
                            available: instance.state.cash as f64,
                        });
                    }
                    instance.state.cash -= switch_cost;
                    line.status = LineStatus::SwitchingTo(switch_time, product);
                    Ok(vec![GameEvent::Message {
                        data: format!(
                            "产线{} 开始转产为 {:?}，费用 {}M，预计 {} 季度完成",
                            line_id, product, switch_cost, switch_time
                        ),
                    }])
                }
            }

            PlayerAction::PurchaseRawMaterial { quantity } => {
                instance.raw_material_order_quantity = quantity;
                Ok(vec![GameEvent::Message {
                    data: format!("原材料订购量设置为 {} 单位（下季度到货）", quantity),
                }])
            }

            PlayerAction::DevelopMarket { market_name } => {
                let dev_cost = 2u32;
                if instance.state.cash < dev_cost {
                    return Err(GameError::InsufficientFunds {
                        required: dev_cost as f64,
                        available: instance.state.cash as f64,
                    });
                }
                let market = instance
                    .state
                    .markets
                    .iter_mut()
                    .find(|m| m.name == market_name)
                    .ok_or_else(|| GameError::InvalidAction(format!("市场不存在: {}", market_name)))?;
                if market.developed {
                    return Err(GameError::InvalidAction(format!(
                        "市场 {} 已开发",
                        market_name
                    )));
                }
                instance.state.cash -= dev_cost;
                market.developed = true;
                Ok(vec![GameEvent::Message {
                    data: format!("市场 {} 开发成功，费用 {}M", market_name, dev_cost),
                }])
            }
        }
    }

    // ── 季度运营 ──

    /// 执行 PDF 季度运营 10 步
    fn execute_quarter(instance: &mut GameInstance) -> Result<Vec<GameEvent>, GameError> {
        let mut events = Vec::new();

        // ── 第 1 步：更新应收账款（账期-1，到期入现金） ──
        {
            let mut cash_in = 0u32;
            let mut new_receivables = Vec::new();
            for ar in instance.state.accounts_receivable.drain(..) {
                if ar.due_quarters <= 1 {
                    cash_in += ar.amount;
                    events.push(GameEvent::Message { data: format!(
                        "应收账款到期入账 {}M",
                        ar.amount
                    ) });
                } else {
                    new_receivables.push(AccountReceivable {
                        amount: ar.amount,
                        due_quarters: ar.due_quarters - 1,
                    });
                }
            }
            instance.state.accounts_receivable = new_receivables;
            instance.state.cash += cash_in;
        }

        // ── 第 2 步：短贷利息（扣利息，检查破产） ──
        {
            let mut interest = 0u32;
            let mut principal = 0u32;
            let mut remaining_loans = Vec::new();
            for mut loan in instance.state.short_term_loans.drain(..) {
                let q_interest = calc_short_term_quarterly_interest(loan.amount);
                interest += q_interest;
                loan.remaining_quarters = loan.remaining_quarters.saturating_sub(1);
                if loan.remaining_quarters == 0 {
                    principal += loan.amount;
                    events.push(GameEvent::Message { data: format!(
                        "短贷到期归还本金 {}M",
                        loan.amount
                    ) });
                } else {
                    remaining_loans.push(loan);
                }
            }
            instance.state.short_term_loans = remaining_loans;
            let total_st = interest + principal;
            if total_st > 0 {
                if instance.state.cash < total_st {
                    return Ok(declare_game_over(
                        &mut instance.state,
                        &mut events,
                        "短贷利息/本金支付导致资金链断裂",
                    ));
                }
                instance.state.cash -= total_st;
                instance.annual_interest_expense += interest;
                events.push(GameEvent::Message { data: format!(
                    "支付短贷利息 {}M，归还本金 {}M",
                    interest, principal
                ) });
            }
        }

        // ── 第 3 步：产品研发推进 ──
        for rd in &mut instance.state.products_rd {
            if rd.completed {
                continue;
            }
            rd.progress += 1;
            if rd.progress >= rd.total {
                rd.completed = true;
                events.push(GameEvent::Message { data: format!(
                    "产品 {:?} 研发完成！",
                    rd.product
                ) });
            } else {
                let rd_cost = product_rd_cost_per_quarter();
                if instance.state.cash < rd_cost {
                    return Ok(declare_game_over(
                        &mut instance.state,
                        &mut events,
                        "研发费用支付导致资金链断裂",
                    ));
                }
                instance.state.cash -= rd_cost;
                instance.annual_rd_expense += rd_cost;
                events.push(GameEvent::Message { data: format!(
                    "支付研发费用 {}M（{:?} {}/{}）",
                    rd_cost, rd.product, rd.progress, rd.total
                ) });
            }
        }

        // ── 第 4 步：供应商交货并付费 ──
        {
            let orders = instance.state.raw_material_orders;
            if orders > 0 {
                let cost = orders; // 1M/单位
                if instance.state.cash < cost {
                    return Ok(declare_game_over(
                        &mut instance.state,
                        &mut events,
                        "原材料采购支付导致资金链断裂",
                    ));
                }
                instance.state.cash -= cost;
                instance.state.raw_material_inventory += orders;
                events.push(GameEvent::Message { data: format!(
                    "供应商交付原材料 {} 单位，支付 {}M",
                    orders, cost
                ) });
                instance.state.raw_material_orders = 0;
            }
        }

        // ── 第 5 步：新原料采购（按玩家设定数量，默认 2） ──
        {
            let order_qty = instance.raw_material_order_quantity.max(1);
            instance.state.raw_material_orders += order_qty;
            events.push(GameEvent::Message { data: format!(
                "采购原材料 {} 单位（下季度到货）",
                order_qty
            ) });
        }

        // ── 第 6 步：更新生产状态（进度+1，完成转成品库） ──
        {
            let mut new_wip = Vec::new();
            for mut wip in instance.state.work_in_progress.drain(..) {
                wip.progress += 1;
                // 生产周期为 2 季度
                if wip.progress >= 2 {
                    // 转入成品库
                    let existing = instance
                        .state
                        .finished_goods
                        .iter_mut()
                        .find(|fg| fg.product == wip.product);
                    if let Some(fg) = existing {
                        fg.quantity += 1;
                    } else {
                        instance.state.finished_goods.push(FinishedGoods {
                            product: wip.product,
                            quantity: 1,
                        });
                    }
                    events.push(GameEvent::Message { data: format!(
                        "产线{} {:?} 生产完成，转入成品库",
                        wip.line_id, wip.product
                    ) });
                } else {
                    new_wip.push(wip);
                }
            }
            instance.state.work_in_progress = new_wip;
        }

        // ── 第 7 步：产线建设/转产推进 ──
        for line in instance.state.factories.iter_mut().flat_map(|f| &mut f.lines) {
            match line.status.clone() {
                LineStatus::Building(remaining) => {
                    if remaining <= 1 {
                        line.status = LineStatus::Idle;
                        events.push(GameEvent::Message { data: format!("产线{} 建设完成", line.id) });
                    } else {
                        line.status = LineStatus::Building(remaining - 1);
                    }
                }
                LineStatus::SwitchingTo(remaining, target) => {
                    if remaining <= 1 {
                        line.product = Some(target);
                        line.status = LineStatus::Idle;
                        events.push(GameEvent::Message { data: format!("产线{} 转产完成", line.id) });
                    } else {
                        line.status = LineStatus::SwitchingTo(remaining - 1, target);
                    }
                }
                _ => {}
            }
        }

        // ── 第 8 步：新生产（空闲产线+有原料→开始生产，付费） ──
        for line in instance.state.factories.iter_mut().flat_map(|f| &mut f.lines) {
            if !matches!(line.status, LineStatus::Idle) {
                continue;
            }
            let Some(product) = line.product else {
                continue;
            };
            if instance.state.raw_material_inventory == 0 {
                events.push(GameEvent::Message { data: format!(
                    "产线{} 原材料不足，无法开始生产",
                    line.id
                ) });
                continue;
            }
            let rm_cost = product.raw_material_cost();
            if instance.state.cash < rm_cost {
                events.push(GameEvent::Message { data: format!(
                    "产线{} 现金不足，无法开始生产",
                    line.id
                ) });
                continue;
            }
            instance.state.cash -= rm_cost;
            instance.state.raw_material_inventory -= 1;
            instance.state.work_in_progress.push(WIPItem {
                line_id: line.id,
                product,
                progress: 0,
            });
            line.status = LineStatus::Producing(product);
            events.push(GameEvent::Message { data: format!(
                "产线{} 开始生产 {:?}",
                line.id, product
            ) });
        }

        // ── 第 9 步：订单交付（有成品→交单→应收账款） ──
        for order in &mut instance.state.orders {
            if order.delivered {
                continue;
            }
            let can_deliver = instance
                .state
                .finished_goods
                .iter()
                .any(|fg| fg.product == order.product && fg.quantity >= order.quantity);
            if !can_deliver {
                continue;
            }
            let fg = instance
                .state
                .finished_goods
                .iter_mut()
                .find(|fg| fg.product == order.product)
                .unwrap();
            fg.quantity -= order.quantity;
            order.delivered = true;
            let total_amount = order.unit_price * order.quantity;
            let cogs =
                (order.product.raw_material_cost() + order.product.production_cost()) * order.quantity;
            instance.annual_revenue += total_amount;
            instance.annual_cost_of_goods += cogs;
            instance.state.accounts_receivable.push(AccountReceivable {
                amount: total_amount,
                due_quarters: order.account_period,
            });
            events.push(GameEvent::Message { data: format!(
                "交付订单 {}，{:?} × {}，金额 {}M，账期 {} 季",
                order.id, order.product, order.quantity, total_amount, order.account_period
            ) });
        }

        // ── 第 10 步：行政管理费 1M（检查破产） ──
        if instance.state.cash < 1 {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "行政管理费支付导致资金链断裂",
            ));
        }
        instance.state.cash -= 1;
        instance.annual_admin_expense += 1;
        events.push(GameEvent::Message { data: "支付行政管理费 1M".to_string() });

        // ── 破产检查：股东权益 < 0 ──
        if calculate_equity(&instance.state) < 0 {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "股东权益为负，游戏结束",
            ));
        }

        Ok(events)
    }

    // ── 年末结算 ──

    /// 执行年末结算
    fn execute_year_end(instance: &mut GameInstance) -> Result<Vec<GameEvent>, GameError> {
        let mut events = Vec::new();
        let year = instance.state.game_year;

        // ── 第 1 步：生产线管理费（1M/条） ──
        let line_count: u32 = instance
            .state
            .factories
            .iter()
            .flat_map(|f| &f.lines)
            .count() as u32;
        let maintenance = line_count;
        if instance.state.cash < maintenance {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "设备维护费支付导致资金链断裂",
            ));
        }
        instance.state.cash -= maintenance;
        events.push(GameEvent::Message { data: format!(
            "支付设备维护费 {}M（{} 条产线）",
            maintenance, line_count
        ) });

        // ── 第 2 步：固定资产折旧 20% ──
        let mut depreciation = 0u32;
        for factory in &mut instance.state.factories {
            let dep = calc_depreciation(factory.value);
            depreciation += dep;
            factory.value = factory.value.saturating_sub(dep);
        }
        events.push(GameEvent::Message { data: format!(
            "固定资产折旧 {}M",
            depreciation
        ) });

        // ── 第 3 步：长贷利息/还本 ──
        let mut lt_interest = 0u32;
        let mut lt_principal = 0u32;
        let mut remaining_slots = Vec::new();
        for slot in instance.state.long_term_loans.drain(..) {
            if !slot.active {
                remaining_slots.push(slot);
                continue;
            }
            let interest = calc_long_term_interest(slot.amount);
            lt_interest += interest;
            // 5 年期限到期还本
            if year >= slot.year + 5 {
                lt_principal += slot.amount;
                events.push(GameEvent::Message { data: format!(
                    "长贷到期归还本金 {}M（{} 年贷款）",
                    slot.amount, slot.year
                ) });
                // 不重新加入，贷款已关闭
            } else {
                remaining_slots.push(slot);
            }
        }
        instance.state.long_term_loans = remaining_slots;
        let total_lt = lt_interest + lt_principal;
        if total_lt > 0 {
            if instance.state.cash < total_lt {
                return Ok(declare_game_over(
                    &mut instance.state,
                    &mut events,
                    "长贷利息/还本导致资金链断裂",
                ));
            }
            instance.state.cash -= total_lt;
            instance.annual_interest_expense += lt_interest;
            events.push(GameEvent::Message { data: format!(
                "支付长贷利息 {}M，归还本金 {}M",
                lt_interest, lt_principal
            ) });
        }

        // ── 第 4 步：税款 ──
        let revenue = instance.annual_revenue;
        let sales_tax = calc_sales_tax(revenue);
        if instance.state.cash < sales_tax {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "营业税支付导致资金链断裂",
            ));
        }
        instance.state.cash -= sales_tax;

        // 利润总额 = 收入 - 所有费用 (不含折旧和维护费？含)
        let total_costs = instance.annual_cost_of_goods
            + instance.annual_admin_expense
            + instance.annual_sales_expense
            + instance.annual_rd_expense
            + depreciation
            + instance.annual_interest_expense
            + instance.annual_discount_fee
            + maintenance;
        let profit_before_tax = revenue as i32 - total_costs as i32;
        let income_tax = calc_income_tax(profit_before_tax);
        if instance.state.cash < income_tax {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "所得税支付导致资金链断裂",
            ));
        }
        instance.state.cash -= income_tax;

        let total_tax = sales_tax + income_tax;
        events.push(GameEvent::Message { data: format!(
            "纳税：营业税 {}M（营收 {}M × 3%），所得税 {}M（利润 {}M × 20%），合计 {}M",
            sales_tax,
            revenue,
            income_tax,
            profit_before_tax.max(0),
            total_tax
        ) });

        // ── 第 5 步：更新 phase ──
        if year >= 8 {
            instance.state.phase = 3;
        } else if year >= 5 {
            instance.state.phase = 2;
        }

        // ── 生成年度财务报告 ──
        let report = generate_annual_report(
            &instance,
            sales_tax,
            income_tax,
            maintenance,
            depreciation,
        );
        instance.annual_reports.push(report.clone());
        events.push(GameEvent::AnnualReport { data: Box::new(report) });

        // ── 第 6 步：破产检查 ──
        if calculate_equity(&instance.state) < 0 {
            return Ok(declare_game_over(
                &mut instance.state,
                &mut events,
                "股东权益为负，游戏结束",
            ));
        }

        Ok(events)
    }
}

// ── 辅助函数 ──

/// 计算股东权益（总资产 - 总负债）
fn calculate_equity(state: &BusinessGameState) -> i32 {
    let cash = state.cash as i32;
    let receivable: i32 = state
        .accounts_receivable
        .iter()
        .map(|a| a.amount as i32)
        .sum();
    let raw_material: i32 = state.raw_material_inventory as i32; // 1M/单位
    let wip: i32 = state.work_in_progress.len() as i32; // 简化估值
    let finished: i32 = state
        .finished_goods
        .iter()
        .map(|fg| fg.quantity as i32)
        .sum();
    let factories: i32 = state.factories.iter().map(|f| f.value as i32).sum();
    let total_assets = cash + receivable + raw_material + wip + finished + factories;

    let long_term: i32 = state
        .long_term_loans
        .iter()
        .filter(|s| s.active)
        .map(|s| s.amount as i32)
        .sum();
    let short_term: i32 = state
        .short_term_loans
        .iter()
        .map(|l| l.amount as i32)
        .sum();
    let total_liabilities = long_term + short_term;

    total_assets - total_liabilities
}

/// 生成年度财务报告
fn generate_annual_report(
    instance: &GameInstance,
    sales_tax: u32,
    income_tax: u32,
    maintenance: u32,
    depreciation: u32,
) -> AnnualReport {
    let year = instance.state.game_year;
    let revenue = instance.annual_revenue;
    let cogs = instance.annual_cost_of_goods;
    let gross_profit = revenue as i32 - cogs as i32;
    let sales_expense = instance.annual_sales_expense;
    let admin_expense = instance.annual_admin_expense;
    let rd_expense = instance.annual_rd_expense;
    let interest_expense = instance.annual_interest_expense;
    let discount_fee = instance.annual_discount_fee;
    let total_tax = sales_tax + income_tax;

    let operating_profit = gross_profit
        - sales_expense as i32
        - admin_expense as i32
        - rd_expense as i32
        - depreciation as i32
        - maintenance as i32;

    let profit_before_tax = operating_profit - interest_expense as i32 - discount_fee as i32;
    let net_profit = profit_before_tax - total_tax as i32;

    // ── 资产负债表 ──
    let cash = instance.state.cash;
    let ar_total: u32 = instance
        .state
        .accounts_receivable
        .iter()
        .map(|a| a.amount)
        .sum();
    let raw_material_value = instance.state.raw_material_inventory; // 1M/单位
    let wip_value = instance.state.work_in_progress.len() as u32;
    let fg_value: u32 = instance
        .state
        .finished_goods
        .iter()
        .map(|fg| fg.quantity)
        .sum();
    let total_current = cash + ar_total + raw_material_value + wip_value + fg_value;

    let factory_value: u32 = instance.state.factories.iter().map(|f| f.value).sum();
    let line_value: u32 = instance
        .state
        .factories
        .iter()
        .flat_map(|f| &f.lines)
        .map(|l| l.line_type.total_build_cost())
        .sum();
    let total_fixed = factory_value + line_value;
    let total_assets = total_current + total_fixed;

    let long_term_total: u32 = instance
        .state
        .long_term_loans
        .iter()
        .filter(|s| s.active)
        .map(|s| s.amount)
        .sum();
    let short_term_total: u32 = instance
        .state
        .short_term_loans
        .iter()
        .map(|l| l.amount)
        .sum();
    let total_liabilities = long_term_total + short_term_total;
    let equity = total_assets as i32 - total_liabilities as i32;

    AnnualReport {
        year,
        income_statement: IncomeStatement {
            revenue,
            cost_of_goods_sold: cogs,
            gross_profit,
            sales_expense,
            admin_expense,
            rd_expense,
            depreciation,
            operating_profit,
            interest_expense,
            discount_fee,
            tax: total_tax,
            net_profit,
        },
        balance_sheet: BalanceSheet {
            cash,
            accounts_receivable: ar_total,
            raw_material: raw_material_value,
            work_in_process: wip_value,
            finished_goods: fg_value,
            total_current_assets: total_current,
            factory: factory_value,
            production_lines: line_value,
            total_fixed_assets: total_fixed,
            total_assets,
            long_term_loans: long_term_total,
            short_term_loans: short_term_total,
            total_liabilities,
            equity,
        },
        expense_sheet: ExpenseSheet {
            new_market_investment: 0,
            product_rd_investment: rd_expense,
            quarterly_admin_fees: admin_expense,
            factory_rent: 0,
            line_maintenance: maintenance,
            product_switch_fees: 0,
            sales_expenses: sales_expense,
            depreciation,
            interest_and_discount: interest_expense + discount_fee,
            tax: total_tax,
        },
    }
}

/// 设置 game_over 标志并返回事件列表（含 game_over 前已产生的事件）
fn declare_game_over(
    state: &mut BusinessGameState,
    events: &mut Vec<GameEvent>,
    reason: &str,
) -> Vec<GameEvent> {
    state.game_over = true;
    state.game_over_reason = Some(reason.to_string());
    events.push(GameEvent::GameOver { data: GameOverData { reason: reason.to_string() } });
    std::mem::take(events)
}

/// 根据营销投入和市场生成订单列表
fn generate_orders(
    marketing_spend: u32,
    market: &Market,
    state: &BusinessGameState,
) -> Vec<Order> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // 每 1M 营销费激活 1 张订单，上限 5
    let count = marketing_spend.min(5) as usize;
    let mut orders = Vec::with_capacity(count);

    // 可用的产品列表（已研发完成的）
    let available_products: Vec<Product> = state
        .products_rd
        .iter()
        .filter(|r| r.completed)
        .map(|r| r.product)
        .collect();

    if available_products.is_empty() {
        return orders;
    }

    for i in 0..count {
        let product = available_products[rng.gen_range(0..available_products.len())];
        // 随机生成订单参数
        let quantity = rng.gen_range(1..=3u32);
        let base_price = match product {
            Product::BenMa => 5u32,
            Product::MengHu => 8,
            Product::FeiYing => 12,
            Product::TianLong => 18,
        };
        // 价格浮动 ±20%
        let price_variation = rng.gen_range(0.8..=1.2);
        let unit_price = ((base_price as f64) * price_variation).round() as u32;
        // 账期 1-4 季度
        let account_period = rng.gen_range(1..=4u32);
        // 10% 概率加急
        let urgent = rng.gen_bool(0.1);

        orders.push(Order {
            id: format!("ORD-{}-{}-{}", market.name, product.name_cn(), i + 1),
            product,
            quantity,
            unit_price,
            account_period,
            delivered: false,
            urgent,
        });
    }

    orders
}

/// 从字符串解析产品类型
fn parse_product(s: &str) -> Option<Product> {
    match s.to_lowercase().as_str() {
        "benma" | "ben_ma" | "奔马" => Some(Product::BenMa),
        "menghu" | "meng_hu" | "猛虎" => Some(Product::MengHu),
        "feiying" | "fei_ying" | "飞鹰" => Some(Product::FeiYing),
        "tianlong" | "tian_long" | "天龙" => Some(Product::TianLong),
        _ => None,
    }
}

/// 从字符串解析产线类型
fn parse_line_type(s: &str) -> Option<LineType> {
    match s.to_lowercase().as_str() {
        "manual" | "手工线" | "手工" => Some(LineType::Manual),
        "semiauto" | "semi_auto" | "半自动线" | "半自动" => Some(LineType::SemiAuto),
        "fullauto" | "full_auto" | "全自动线" | "全自动" => Some(LineType::FullAuto),
        _ => None,
    }
}
