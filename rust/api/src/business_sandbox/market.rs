/// 市场与订单模块
///
/// 提供创业沙盘游戏中市场订单相关的计算函数。
/// 所有金额单位均为 "M"（百万），使用 `u32` 表示。

use crate::business_sandbox::state::Market;

// ── 订单激活 ──

/// 计算激活的订单数量
///
/// 每 1M 营销投入激活 1 张订单，上限为 `max_orders`。
#[inline]
pub fn calc_activated_orders(marketing_spend: u32, max_orders: u32) -> u32 {
    marketing_spend.min(max_orders)
}

// ── 延期交付罚款 ──

/// 计算延期交付罚款金额
///
/// 延期交付需支付原金额的 75%，四舍五入取整。
#[inline]
pub fn calc_late_delivery_penalty(original_amount: u32) -> u32 {
    (original_amount * 3 + 2) / 4
}

// ── 选单顺序 ──

/// 计算选单顺序
///
/// 排名第1的（rank=1）优先选单，其余按营销投入降序排列。
/// 返回原始数组的索引列表。
pub fn calc_selection_order(companies: &[(u32, u32)]) -> Vec<usize> {
    if companies.is_empty() {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..companies.len()).collect();

    // 找到市场领导者（rank=1）
    if let Some(pos) = indices.iter().position(|&i| companies[i].0 == 1) {
        let leader = indices.remove(pos);
        // 其余按 marketing_spend 降序排列
        indices.sort_by(|&a, &b| companies[b].1.cmp(&companies[a].1));
        indices.insert(0, leader);
    } else {
        indices.sort_by(|&a, &b| companies[b].1.cmp(&companies[a].1));
    }

    indices
}

// ── 默认市场列表 ──

/// 创建默认市场列表
///
/// 包含所有可选市场，其中"平城"市场初始为已开发状态。
pub fn create_default_markets() -> Vec<Market> {
    vec![
        Market {
            name: "平城".to_string(),
            developed: true,
            rank: 0,
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
    ]
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activated_orders() {
        assert_eq!(calc_activated_orders(3, 5), 3);
        assert_eq!(calc_activated_orders(10, 5), 5);
        assert_eq!(calc_activated_orders(0, 5), 0);
        assert_eq!(calc_activated_orders(5, 5), 5);
    }

    #[test]
    fn test_late_delivery_penalty() {
        assert_eq!(calc_late_delivery_penalty(100), 75);
        assert_eq!(calc_late_delivery_penalty(10), 8); // 7.5 -> 8
        assert_eq!(calc_late_delivery_penalty(1), 1);  // 0.75 -> 1
        assert_eq!(calc_late_delivery_penalty(0), 0);
    }

    #[test]
    fn test_selection_order() {
        // company 0: rank=1, spend=3  (market leader - picks first)
        // company 1: rank=3, spend=5  (highest spender)
        // company 2: rank=2, spend=2  (lower spender)
        let companies = vec![(1, 3), (3, 5), (2, 2)];
        let order = calc_selection_order(&companies);
        assert_eq!(order[0], 0, "Market leader should pick first");
        // After leader, sorted by marketing spend descending
        assert_eq!(order[1], 1, "Highest spender should pick second");
        assert_eq!(order[2], 2, "Lowest spender should pick last");
    }

    #[test]
    fn test_selection_order_no_leader() {
        let companies = vec![(2, 3), (3, 5), (4, 2)];
        let order = calc_selection_order(&companies);
        // No rank=1, so sorted by spend descending
        assert_eq!(order[0], 1, "Highest spender should pick first");
        assert_eq!(order[1], 0, "Middle spender should pick second");
        assert_eq!(order[2], 2, "Lowest spender should pick last");
    }

    #[test]
    fn test_selection_order_empty() {
        let companies: Vec<(u32, u32)> = vec![];
        let order = calc_selection_order(&companies);
        assert!(order.is_empty());
    }

    #[test]
    fn test_create_default_markets() {
        let markets = create_default_markets();
        assert!(!markets.is_empty(), "Should have at least one market");
        let pingcheng = markets.iter().find(|m| m.name == "平城");
        assert!(pingcheng.is_some(), "Should include 平城 market");
        assert!(pingcheng.unwrap().developed, "平城 should be developed");
    }
}
