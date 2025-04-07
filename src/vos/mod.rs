use serde::Serialize;

use crate::gitmodule::CommitInfo;

pub mod userdata;

#[derive(Debug, Serialize)]
pub struct ReposVo {
    pub name: String,
    // pub path: String,
    // pub last_commit: Option<CommitInfo>,
    pub branch: String,
}
