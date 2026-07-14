use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::business_sandbox::state::*;
use crate::state::AppState;

/// WebSocket 路由处理函数
pub async fn game_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_game_ws(socket, state, game_id))
}

async fn handle_game_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    game_id: String,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(256);

    // 注册到游戏管理器
    state.business_sandbox.register_ws(&game_id, tx).await;

    // 创建游戏
    if state.business_sandbox.manager.create_game(&game_id).await.is_err() {
        return;
    }

    // 发送初始状态 + 要求竞标决策
    if let Ok(gs) = state.business_sandbox.manager.get_state(&game_id).await {
        let msg = serde_json::to_string(&GameEvent::StateUpdate { data: Box::new(gs) }).unwrap();
        let _ = sender.send(Message::Text(msg)).await;
    }
    let ask = serde_json::to_string(&GameEvent::AskDecision {
        data: AskDecisionData {
            year: 1,
            quarter: 0,
            decision_type: "bidding".into(),
        },
    })
    .unwrap();
    let _ = sender.send(Message::Text(ask)).await;

    // 发送任务：转发 mpsc 到 WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 接收任务：处理客户端消息
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(action) = serde_json::from_str::<PlayerAction>(&text) {
                match state
                    .business_sandbox
                    .manager
                    .handle_action(&game_id, action)
                    .await
                {
                    Ok(events) => {
                        for event in events {
                            let json = serde_json::to_string(&event).unwrap();
                            let _ = state
                                .business_sandbox
                                .broadcast(&game_id, &json)
                                .await;
                        }
                    }
                    Err(e) => {
                        let err_msg = serde_json::to_string(
                            &GameEvent::Message { data: format!("错误: {}", e) },
                        )
                        .unwrap();
                        let _ = state
                            .business_sandbox
                            .broadcast(&game_id, &err_msg)
                            .await;
                    }
                }
            }
        }
    }

    send_task.abort();
}
