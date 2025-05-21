use chrono::DateTime;
use serde::Serialize;

use crate::models::message::{Message, MessageStatus, MessageType};
pub mod userdata;

#[derive(Debug, Serialize)]
pub struct ReposVo {
    pub name: String,
    // pub path: String,
    // pub last_commit: Option<CommitInfo>,
    pub branch: String,
}

#[derive(Debug, Serialize)]
pub struct UserMsg {
    pub message_type: MessageType,
    pub content: String,
    pub read_status: MessageStatus,
    pub created_at: DateTime<chrono::Utc>,
}
