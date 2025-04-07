use chrono::{DateTime, Utc};
use git2::{Error as GitError, Repository};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub time: DateTime<Utc>,
}

pub fn get_commit_history(repo_path: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;

    // 配置遍历器从HEAD开始
    revwalk.push_head()?;

    let mut commit_history = Vec::new();

    for (i, oid_result) in revwalk.enumerate() {
        // 如果达到请求的记录数上限，则停止
        if i >= limit {
            break;
        }

        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let author = commit.author();
        let time = commit.time();

        // 将git提交时间转换为DateTime<Utc>
        let dt = DateTime::<Utc>::from_timestamp(time.seconds(), 0).unwrap_or_else(|| Utc::now());

        let commit_info = CommitInfo {
            id: commit.id().to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            email: author.email().unwrap_or("Unknown").to_string(),
            message: commit.message().unwrap_or("No message").to_string(),
            time: dt,
        };

        commit_history.push(commit_info);
    }

    Ok(commit_history)
}
