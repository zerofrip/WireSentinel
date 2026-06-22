use crate::api::middleware::{ws_connection_acquired, ws_connection_released};
use crate::api::AppState;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    if ws_connection_acquired().is_err() {
        return;
    }

    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.deps.events.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => { recv_task.abort(); },
        _ = &mut recv_task => { send_task.abort(); },
    }

    ws_connection_released();
}
