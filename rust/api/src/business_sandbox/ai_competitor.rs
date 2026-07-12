use std::sync::Arc;
use serde::{Serialize, Deserialize};

use ai_gateway::GatewayRegistry;
use registry::SoulRegistry;
use foundation::{Prompt, PromptMessage, CallConfig};

use crate::business_sandbox::state::*;

/// AI 竞争对手的年度决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorDecision {
    pub company_name: String,
    pub marketing_spend: u32,
    pub product_focus: Product,
    pub pricing_strategy: String, // "aggressive" | "normal" | "premium"
}

/// 生成 AI 竞争对手决策
/// 使用现有 AI 网关和 Soul 注册表
/// 如果 AI 调用失败，使用合理的默认值
pub async fn generate_competitor_decisions(
    gateway: Arc<GatewayRegistry>,
    registry: Arc<SoulRegistry>,
    state: &BusinessGameState,
) -> Vec<CompetitorDecision> {
    let competitor_names = vec![
        "CEO", "CFO", "COO", "CMO", "管理咨询顾问",
    ];

    let mut decisions = Vec::new();
    let market_summary = format_market_summary(state);

    for name in &competitor_names {
        let decision = generate_single_competitor(
            gateway.clone(),
            registry.clone(),
            name,
            &market_summary,
        )
        .await;

        decisions.push(decision);
    }

    decisions
}

/// 生成单个竞争对手的决策
async fn generate_single_competitor(
    gateway: Arc<GatewayRegistry>,
    registry: Arc<SoulRegistry>,
    soul_name: &str,
    market_summary: &str,
) -> CompetitorDecision {
    // 尝试从注册表获取灵魂配置
    if let Ok(profile) = registry.get_soul(soul_name) {
        let prompt_text = build_competitor_prompt(soul_name, market_summary);

        // 尝试挑选 AI provider
        if let Some(provider) = gateway.pick_provider() {
            if let Some(gw) = gateway.get(&provider) {
                let call_config = CallConfig {
                    temperature: 0.7,
                    max_tokens: 300,
                    stream: false,
                    model: Some(profile.model.clone()),
                    ..Default::default()
                };

                let prompt = Prompt {
                    messages: vec![
                        PromptMessage {
                            role: "system".into(),
                            content: profile.summon_prompt.clone(),
                            reasoning_content: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        PromptMessage {
                            role: "user".into(),
                            content: prompt_text,
                            reasoning_content: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                    ],
                };

                let mut rx = gw.call(&prompt, &call_config);
                let mut response = String::new();
                while let Some(Ok(chunk)) = rx.recv().await {
                    response.push_str(&chunk.content);
                }

                // 尝试解析 JSON 响应
                if let Ok(decision) = serde_json::from_str::<CompetitorDecision>(&response) {
                    return decision;
                }
            }
        }
    }

    // 回退：返回合理默认值
    CompetitorDecision {
        company_name: soul_name.to_string(),
        marketing_spend: 2,
        product_focus: Product::BenMa,
        pricing_strategy: "normal".into(),
    }
}

/// 构建给 AI 的市场摘要
fn format_market_summary(state: &BusinessGameState) -> String {
    format!(
        "当前第{}年，第{}季度。现金: {}M。\
         现有市场: {}。\
         当前阶段: {}。",
        state.game_year,
        state.game_quarter,
        state.cash,
        state
            .markets
            .iter()
            .filter(|m| m.developed)
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        state.phase,
    )
}

/// 构建 AI 竞争对手提示词
fn build_competitor_prompt(soul_name: &str, market_summary: &str) -> String {
    format!(
        "你是一家市场上竞争公司的{role}。\n\
         当前市场情况：{market_summary}\n\n\
         请做出本年度的经营决策，以JSON格式输出：\n\
         {{\n\
           \"company_name\": \"{name}\",\n\
           \"marketing_spend\": <整数，营销投入M>,\n\
           \"product_focus\": \"ben_ma|meng_hu|fei_ying|tian_long\",\n\
           \"pricing_strategy\": \"aggressive|normal|premium\"\n\
         }}",
        role = soul_name,
        market_summary = market_summary,
        name = soul_name,
    )
}

/// 模拟定价对需求的影响
pub fn apply_pricing_effect(
    base_price: u32,
    strategy: &str,
) -> f64 {
    match strategy {
        "aggressive" => base_price as f64 * 0.85, // 降价15%抢市场
        "premium" => base_price as f64 * 1.15,     // 溢价15%
        _ => base_price as f64,                     // 正常定价
    }
}
