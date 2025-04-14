use std::{collections::HashMap, hash::Hash, sync::Arc};

use futures::channel::mpsc::Sender;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info};

#[derive(Debug, Serialize)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub message: String,
    pub time: i64,
}

#[derive(Debug, Serialize)]
pub struct GitOperationResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct GitFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>, // 仅对文件有效
    pub children: Vec<GitFileEntry>,
}

#[derive(Debug, Serialize)]
pub struct CommitFileChange {
    pub path: String,
    pub status: String,       // "added", "modified", "deleted" 等
    pub diff: Option<String>, // 文件差异内容
}

#[derive(Debug, Serialize)]
pub struct CommitDetail {
    pub commit_info: CommitInfo,
    pub file_changes: Vec<CommitFileChange>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebSocketMsg {
    pub user_id: String,
    pub repo_name: String,
    pub message: String,
}

pub struct WebSocketSender {
    tx: mpsc::UnboundedSender<WebSocketMsg>,
}

#[derive(Clone)]
pub struct WebSocketManager {
    connections: Arc<Mutex<HashMap<String, Vec<WebSocketSender>>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        WebSocketManager {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_connection(
        &self,
        user_id: &str,
        tx: mpsc::UnboundedSender<WebSocketMsg>,
    ) {
        let mut connections = self.connections.lock().await;
        let entry = connections
            .entry(user_id.to_string())
            .or_insert_with(Vec::new);
        entry.push(WebSocketSender { tx });
        info!("Registered new WebSocket connection for user: {}", user_id);
    }

    pub async fn unregister_connection(
        &self,
        user_id: &str,
        tx: &mpsc::UnboundedSender<WebSocketMsg>,
    ) {
        let mut connections = self.connections.lock().await;

        if let Some(senders) = connections.get_mut(user_id) {
            // 删除和senders里面 tx和要删除的tx一样的
            senders.retain(|sender| !sender.tx.same_channel(tx));
            if senders.is_empty() {
                connections.remove(user_id);
            }
        }
        info!("Unregistered WebSocket connection for user: {}", user_id);
    }

    pub async fn send_message(&self, user_id: &str, message: WebSocketMsg) {
        let connections = self.connections.lock().await;
        if let Some(senders) = connections.get(user_id) {
            for sender in senders {
                if let Err(e) = sender.tx.send(message.clone()) {
                    error!("Failed to send WebSocket Message:{}", e);
                }
            }
        }
    }

    pub async fn send_clone_status(&self, user_id: &str, repo_name: &str, status: &str) {
        let message = WebSocketMsg {
            user_id: user_id.to_string(),
            repo_name: repo_name.to_string(),
            message: status.to_string(),
        };
        self.send_message(user_id, message).await;
        info!(
            "Sent clone status for user: {}, repo: {}, status: {}",
            user_id, repo_name, status
        );
    }
}
