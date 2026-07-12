use serde::{Deserialize, Serialize};
use std::fmt;

/// 创业沙盘游戏错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameError {
    /// 游戏不存在
    GameNotFound(String),
    /// 无效的操作
    InvalidAction(String),
    /// 资金不足
    InsufficientFunds {
        required: f64,
        available: f64,
    },
    /// 产能不足
    InsufficientCapacity(String),
    /// 研发未完成
    RDAIncomplete(String),
    /// 市场未开发
    MarketNotDeveloped(String),
    /// 无效的订单
    InvalidOrder(String),
    /// 无效的阶段
    WrongPhase(String),
    /// 游戏已结束
    GameAlreadyOver,
    /// 内部错误
    Internal(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::GameNotFound(id) => write!(f, "游戏不存在: {}", id),
            GameError::InvalidAction(msg) => write!(f, "无效操作: {}", msg),
            GameError::InsufficientFunds { required, available } => {
                write!(f, "资金不足: 需要 {}, 可用 {}", required, available)
            }
            GameError::InsufficientCapacity(msg) => write!(f, "产能不足: {}", msg),
            GameError::RDAIncomplete(msg) => write!(f, "研发未完成: {}", msg),
            GameError::MarketNotDeveloped(msg) => write!(f, "市场未开发: {}", msg),
            GameError::InvalidOrder(msg) => write!(f, "无效订单: {}", msg),
            GameError::WrongPhase(msg) => write!(f, "非当前阶段操作: {}", msg),
            GameError::GameAlreadyOver => write!(f, "游戏已结束"),
            GameError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for GameError {}
