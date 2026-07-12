/// 财务计算模块
///
/// 提供创业沙盘游戏中各项财务计算函数。
/// 所有金额单位均为 "M"（百万），使用 `u32` 表示。

// ── 贴现 ──

/// 计算贴现手续费
///
/// 公式：`amount × 1/14`，四舍五入取整。
#[inline]
pub fn calc_discount_fee(amount: u32) -> u32 {
    (amount + 7) / 14
}

// ── 折旧 ──

/// 计算固定资产折旧
///
/// 公式：`value × 20%`，四舍五入取整。
#[inline]
pub fn calc_depreciation(value: u32) -> u32 {
    (value + 2) / 5
}

// ── 利息 ──

/// 计算长期贷款年息
///
/// 公式：`principal × 5%`，四舍五入取整。
#[inline]
pub fn calc_long_term_interest(principal: u32) -> u32 {
    (principal + 10) / 20
}

/// 计算短期贷款季度利息
///
/// 公式：`principal × 10% / 4`，四舍五入取整。
#[inline]
pub fn calc_short_term_quarterly_interest(principal: u32) -> u32 {
    (principal + 20) / 40
}

// ── 税金 ──

/// 计算营业税
///
/// 公式：`revenue × 3%`，四舍五入取整。
#[inline]
pub fn calc_sales_tax(revenue: u32) -> u32 {
    (revenue * 3 + 50) / 100
}

/// 计算所得税
///
/// 公式：`profit_before_tax × 20%`，四舍五入取整。
/// 利润 ≤ 0 时返回 0。
#[inline]
pub fn calc_income_tax(profit_before_tax: i32) -> u32 {
    if profit_before_tax <= 0 {
        0
    } else {
        ((profit_before_tax as u32) + 2) / 5
    }
}

// ── 贷款 ──

/// 计算贷款额度
///
/// 贷款总额度 = `net_assets × 4`，其中长贷 2 倍、短贷 2 倍。
/// `net_assets ≤ 0` 时返回 `(0, 0)`。
///
/// 返回值 `(long_term_limit, short_term_limit)`。
#[inline]
pub fn calc_loan_limit(net_assets: i32) -> (u32, u32) {
    if net_assets <= 0 {
        (0, 0)
    } else {
        let n = net_assets as u32;
        (n * 2, n * 2)
    }
}

/// 校验贷款金额是否合法
///
/// 最低 20M，且为 20M 的倍数。
#[inline]
pub fn validate_loan_amount(amount: u32) -> Result<u32, String> {
    if amount < 20 {
        return Err(format!(
            "贷款金额不能低于 20M，当前为 {}M",
            amount
        ));
    }
    if amount % 20 != 0 {
        return Err(format!(
            "贷款金额必须是 20M 的倍数，当前为 {}M",
            amount
        ));
    }
    Ok(amount)
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discount_fee() {
        assert_eq!(calc_discount_fee(14), 1);
        assert_eq!(calc_discount_fee(28), 2);
        assert_eq!(calc_discount_fee(13), 1);
        assert_eq!(calc_discount_fee(7), 1); // 0.5 rounds to 1
    }

    #[test]
    fn test_depreciation() {
        assert_eq!(calc_depreciation(20), 4);
        assert_eq!(calc_depreciation(10), 2);
        assert_eq!(calc_depreciation(16), 3);
        assert_eq!(calc_depreciation(5), 1);
    }

    #[test]
    fn test_long_term_interest() {
        assert_eq!(calc_long_term_interest(40), 2);
        assert_eq!(calc_long_term_interest(20), 1);
    }

    #[test]
    fn test_short_term_quarterly_interest() {
        assert_eq!(calc_short_term_quarterly_interest(40), 1);
        assert_eq!(calc_short_term_quarterly_interest(20), 1);
    }

    #[test]
    fn test_sales_tax() {
        assert_eq!(calc_sales_tax(100), 3);
        assert_eq!(calc_sales_tax(50), 2);
    }

    #[test]
    fn test_income_tax() {
        assert_eq!(calc_income_tax(50), 10);
        assert_eq!(calc_income_tax(-10), 0);
        assert_eq!(calc_income_tax(0), 0);
        assert_eq!(calc_income_tax(13), 3);
    }

    #[test]
    fn test_loan_limit() {
        assert_eq!(calc_loan_limit(100), (200, 200));
        assert_eq!(calc_loan_limit(0), (0, 0));
        assert_eq!(calc_loan_limit(-10), (0, 0));
    }

    #[test]
    fn test_validate_loan_amount() {
        assert!(validate_loan_amount(20).is_ok());
        assert!(validate_loan_amount(40).is_ok());
        assert!(validate_loan_amount(10).is_err());
        assert!(validate_loan_amount(30).is_err());
    }
}
