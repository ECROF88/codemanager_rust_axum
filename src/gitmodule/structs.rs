use serde::{Deserialize, Serialize};

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
