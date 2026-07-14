/// 生产与供应链模块
///
/// 提供创业沙盘游戏中产品研发、原材料采购与生产成本计算、产线残值等函数。
/// 所有金额单位均为 "M"（百万），使用 `u32` 表示。

use crate::business_sandbox::state::{LineType, Product};

// ── 产品研发 ──

/// 返回指定产品所需的研发总季度数
///
/// * `BenMa` — 0（已有，无需研发）
/// * `MengHu` — 6
/// * `FeiYing` — 8
/// * `TianLong` — 基础 14；若 `has_feijing` 为 `true`（已研发飞鹰），减 5 为 9
pub fn product_rd_total_quarters(product: Product, has_feijing: bool) -> u32 {
    match product {
        Product::BenMa => 0,
        Product::MengHu => 6,
        Product::FeiYing => 8,
        Product::TianLong => {
            if has_feijing {
                9 // 14 - 5
            } else {
                14
            }
        }
    }
}

/// 每季度研发费用（固定 1M）
#[inline]
pub fn product_rd_cost_per_quarter() -> u32 {
    1
}

// ── 原材料成本 ──

/// 返回指定产品的单位原材料成本
///
/// 委派到 [`Product::raw_material_cost`]。
#[inline]
pub fn raw_material_cost(product: Product) -> u32 {
    product.raw_material_cost()
}

// ── 生产成本 ──

/// 返回指定产品的单位生产成本
///
/// 委派到 [`Product::production_cost`]。
#[inline]
pub fn production_cost(product: Product) -> u32 {
    product.production_cost()
}

// ── 产线残值 ──

/// 返回指定产线类型的残值
///
/// 委派到 [`LineType::salvage_value`]。
#[inline]
pub fn line_salvage_value(line_type: LineType) -> u32 {
    line_type.salvage_value()
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business_sandbox::state::{LineType, Product};

    #[test]
    fn test_product_rd_quarters() {
        assert_eq!(product_rd_total_quarters(Product::BenMa, false), 0);
        assert_eq!(product_rd_total_quarters(Product::MengHu, false), 6);
        assert_eq!(product_rd_total_quarters(Product::FeiYing, false), 8);
        assert_eq!(product_rd_total_quarters(Product::TianLong, false), 14);
        assert_eq!(product_rd_total_quarters(Product::TianLong, true), 9);
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
    fn test_line_salvage() {
        assert_eq!(line_salvage_value(LineType::Manual), 2);
        assert_eq!(line_salvage_value(LineType::FullAuto), 6);
    }
}
