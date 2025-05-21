use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, prelude::Type};

/// 消息状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[sqlx(type_name = "messagestatus", rename_all = "lowercase")]
pub enum MessageStatus {
    Unread,
    Read,
}

impl Default for MessageStatus {
    fn default() -> Self {
        MessageStatus::Unread
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    System,
    Notification,
    Alert,
    RepoUpdate,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::System
    }
}
impl ToString for MessageType {
    fn to_string(&self) -> String {
        match self {
            MessageType::System => "system".to_string(),
            MessageType::Notification => "notification".to_string(),
            MessageType::Alert => "alert".to_string(),
            MessageType::RepoUpdate => "repoupdate".to_string(),
        }
    }
}
/// 消息实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub message_id: i32,
    pub username: String,
    pub content: String,
    // #[sqlx(try_from = "String")]
    pub status: MessageStatus, // 数据库里面用了枚举类型
    #[sqlx(try_from = "String", rename = "type")]
    pub message_type: MessageType,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreate {
    pub user_id: i32,
    pub content: String,
    pub message_type: MessageType,
}

impl TryFrom<String> for MessageStatus {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "unread" => Ok(MessageStatus::Unread),
            "read" => Ok(MessageStatus::Read),
            _ => Err(format!("Invalid message status: {}", s)),
        }
    }
}

/// 实现从字符串转换为消息类型的功能
impl TryFrom<String> for MessageType {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "system" => Ok(MessageType::System),
            "notification" => Ok(MessageType::Notification),
            "alert" => Ok(MessageType::Alert),
            "repoupdate" => Ok(MessageType::RepoUpdate),
            _ => Err(format!("Invalid message type: {}", s)),
        }
    }
}

impl Message {
    /// 创建一个新的未读系统消息
    pub fn new_system_message(
        user_id: i32,
        content: String,
        msg_type: Option<MessageType>,
    ) -> MessageCreate {
        MessageCreate {
            user_id,
            content,
            message_type: msg_type.unwrap_or(MessageType::default()),
        }
    }

    /// 检查消息是否已读
    pub fn is_read(&self) -> bool {
        self.status == MessageStatus::Read
    }

    /// 将消息标记为已读
    pub fn mark_as_read(&mut self) {
        self.status = MessageStatus::Read;
        self.read_at = Some(Utc::now());
    }
}
