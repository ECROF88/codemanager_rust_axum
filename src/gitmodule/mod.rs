use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{shared::error::AppError, vos::ReposVo};
// use axum::extract::Path;
use config::Config;
use git2::{IndexAddOption, Repository, Signature, build::RepoBuilder};
use hyper::header;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitConfig {
    pub name: String,
    pub email: String,
}

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

#[derive(Clone)]
pub struct GitManager {
    base_path: PathBuf, // 仓库存储的基础路径
                        // config: GitConfig,  // 默认Git配置
}

impl GitManager {
    pub fn new(
        base_path: &str,
        //  config: GitConfig
    ) -> Self {
        let path = PathBuf::from(base_path);

        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create base directory");
        }

        GitManager {
            base_path: path,
            // config,
        }
    }

    pub fn get_user_repo_path(&self, user_id: &str, repo_name: &str) -> PathBuf {
        self.base_path.join(user_id).join(repo_name)
    }

    fn ensure_user_directory(&self, user_id: &str) -> Result<PathBuf, AppError> {
        let user_path = self.base_path.join(user_id);

        if !user_path.exists() {
            fs::create_dir_all(&user_path).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create user directory: {}", e))
            })?;
        }

        Ok(user_path)
    }

    // repo_url:remote repo url
    // repo_name:remote url clone into local, and give a name
    // base_path:local path
    pub async fn clone_repository_for_user(
        &self,
        user_id: &str,
        repo_url: &str,
        repo_name: &str,
    ) -> Result<String, AppError> {
        let user_path = self.ensure_user_directory(user_id)?;
        let repo_path = user_path.join(repo_name);

        // 检查目标目录是否已存在
        if repo_path.exists() {
            return Err(AppError::BadRequest(format!(
                "Repository {} already exists for user {}",
                repo_name, user_id
            )));
        }
        // 在专用线程池中执行阻塞操作
        let repo_path_clone = repo_path.clone();
        let repo_url = repo_url.to_string();
        let r = tokio::task::spawn_blocking(move || {
            // 此操作在单独的线程中执行，不会阻塞异步运行时
            RepoBuilder::new()
                .clone(&repo_url, &repo_path_clone)
                .map_err(|e| e)
                .map(|_| repo_path_clone.to_string_lossy().to_string())
        })
        .await
        .map_err(|e| AppError::InternalServerError(format!("Thread panicked: {:?}", e)))?
        .map_err(|e| AppError::InternalServerError(format!("Clone failed: {}", e)))?; // 等待线程完成并解包结果

        // Ok(repo_path.to_string_lossy().to_string())
        Ok(r)

        // // 执行克隆操作
        // match RepoBuilder::new().clone(repo_url, &repo_path) {
        //     Ok(_) => {
        //         println!("{:?}", repo_path);
        //         Ok(repo_path.to_string_lossy().to_string())
        //     }
        //     Err(e) => Err(AppError::InternalServerError(format!(
        //         "Clone failed: {}",
        //         e
        //     ))),
        // }
    }

    fn open_repo(&self, repo_path: &Path) -> Result<Repository, AppError> {
        Repository::open(repo_path)
            .map_err(|e| AppError::InternalServerError(format!("Failed to open repository: {}", e)))
    }

    pub fn commit_for_user(
        &self,
        user_id: &str,
        repo_name: &str,
        message: &str,
        paths: &[&str],
        // user_name: &str,  // 用户姓名，通常是 username
        user_email: &str, // 用户邮箱
    ) -> Result<String, AppError> {
        let repo_path = self.get_user_repo_path(user_id, repo_name);
        let repo = self.open_repo(&repo_path)?;

        // 将文件添加到索引
        let mut index = repo
            .index()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get index: {}", e)))?;

        if paths.is_empty() {
            // 添加所有更改
            index
                .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
                .map_err(|e| {
                    AppError::InternalServerError(format!("Failed to add files: {}", e))
                })?;
        } else {
            // 添加特定文件

            for path in paths {
                index.add_path(Path::new(path)).map_err(|e| {
                    AppError::InternalServerError(format!("Failed to add {}: {}", path, e))
                })?;
            }
        }

        // 写入索引
        let oid = index
            .write_tree()
            .map_err(|e| AppError::InternalServerError(format!("Failed to write index: {}", e)))?;

        // 索引写入磁盘
        index.write().map_err(|e| {
            AppError::InternalServerError(format!("Failed to write index file: {}", e))
        })?;

        // 创建提交
        // let signature = Signature::now(&self.config.name, &self.config.email).map_err(|e| {
        //     AppError::InternalServerError(format!("Failed to create signature: {}", e))
        // })?;
        let signature = Signature::now(user_id, user_email).map_err(|e| {
            AppError::InternalServerError(format!("Failed to create signature: {}", e))
        })?;

        let tree = repo
            .find_tree(oid)
            .map_err(|e| AppError::InternalServerError(format!("Failed to find tree: {}", e)))?;

        // // 获取父提交
        // let parent_commit = match repo.head() {
        //     Ok(head) => Some(head.peel_to_commit().map_err(|e| {
        //         AppError::InternalServerError(format!("Failed to get parent commit: {}", e))
        //     })?),
        //     Err(_) => None,
        // };
        let parent_commit = match repo.head() {
            Ok(head) => {
                // Peel the reference to the underlying commit object
                match head.peel_to_commit() {
                    Ok(commit) => Some(commit),
                    Err(e) => {
                        return Err(AppError::InternalServerError(format!(
                            "Failed to peel HEAD to commit: {}",
                            e
                        )));
                    }
                }
            }
            Err(e)
                if e.code() == git2::ErrorCode::UnbornBranch
                    || e.code() == git2::ErrorCode::NotFound =>
            {
                // This is the initial commit, no parent.
                None
            }
            Err(e) => {
                // Handle other unexpected errors when accessing HEAD
                return Err(AppError::InternalServerError(format!(
                    "Failed to get HEAD reference: {}",
                    e
                )));
            }
        };

        // Parents slice needs to be &[&Commit]
        let parents_vec: Vec<&git2::Commit> =
            parent_commit.as_ref().map_or(Vec::new(), |c| vec![c]);
        let parents_slice: &[&git2::Commit] = &parents_vec;

        // 创建提交
        let commit_id = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                parents_slice,
            )
            .map_err(|e| AppError::InternalServerError(format!("Failed to commit: {}", e)))?;

        Ok(commit_id.to_string())
    }

    pub fn get_commit_history(
        &self,
        user_id: &str,
        repo_name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, AppError> {
        // bsaepath/user_id/repo_name
        let repo_path = self.base_path.join(user_id).join(repo_name);
        let repo = self.open_repo(&repo_path)?;

        let mut revwalk = repo.revwalk().map_err(|e| {
            AppError::InternalServerError(format!("Failed to create revwalk: {}", e))
        })?;

        revwalk
            .push_head()
            .map_err(|e| AppError::InternalServerError(format!("Failed to push head: {}", e)))?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }

            let oid = oid.map_err(|e| {
                AppError::InternalServerError(format!("Failed to get commit ID: {}", e))
            })?;
            let commit = repo.find_commit(oid).map_err(|e| {
                AppError::InternalServerError(format!("Failed to find commit: {}", e))
            })?;

            commits.push(CommitInfo {
                id: commit.id().to_string(),
                author: format!(
                    "{} <{}>",
                    commit.author().name().unwrap_or(""),
                    commit.author().email().unwrap_or("")
                ),
                message: commit.message().unwrap_or("").to_string(),
                time: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    pub fn get_repo_status(&self, repo_name: &str) -> Result<Vec<String>, AppError> {
        let repo_path = self.base_path.join(repo_name);
        let repo = self.open_repo(&repo_path)?;

        let mut status = Vec::new();
        let statuses = repo
            .statuses(None)
            .map_err(|e| AppError::InternalServerError(format!("Failed to get status: {}", e)))?;

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("");
            let status_str = format!("{}: {}", path, self.status_to_string(entry.status()));
            status.push(status_str);
        }

        Ok(status)
    }

    pub fn get_repos_data_for_users(&self, user_id: &str) -> Result<Vec<ReposVo>, AppError> {
        println!("userid{} is getting all repos data", user_id);
        let user_dir = self.base_path.join(user_id);
        println!("user_dir is {:?}", user_dir);
        if !user_dir.exists() {
            return Ok(Vec::new()); // 用户没有仓库，返回空列表
        }

        let entries = fs::read_dir(&user_dir).map_err(|e| {
            AppError::InternalServerError(format!("Failed to read user directory: {}", e))
        })?;

        let mut repos = Vec::new();

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                println!("each entry.path is {:?}", path);
                if path.is_dir() && Repository::open(&path).is_ok() {
                    let branch = self.get_current_branch(&path)?;
                    let repo_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();

                    repos.push(ReposVo {
                        name: repo_name,
                        branch,
                    });
                }
            }
        }
        Ok(repos)
    }

    fn get_current_branch(&self, repo_path: &Path) -> Result<String, AppError> {
        let repo = self.open_repo(repo_path)?;

        let head = repo.head().map_err(|e| {
            AppError::InternalServerError(format!("Failed to get HEAD reference: {}", e))
        })?;

        if head.is_branch() {
            let branch_name = head.shorthand().unwrap_or("unknown").to_string();
            Ok(branch_name)
        } else {
            // 可能是detached HEAD状态
            let commit_id = head
                .peel_to_commit()
                .map(|c| c.id().to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            Ok(format!("detached@{}", &commit_id[..7]))
        }
    }

    // 辅助方法：状态码转字符串
    fn status_to_string(&self, status: git2::Status) -> String {
        let mut status_str = String::new();

        if status.is_index_new() {
            status_str.push_str("新增索引 ");
        }
        if status.is_index_modified() {
            status_str.push_str("修改索引 ");
        }
        if status.is_index_deleted() {
            status_str.push_str("删除索引 ");
        }
        if status.is_wt_new() {
            status_str.push_str("新增工作区 ");
        }
        if status.is_wt_modified() {
            status_str.push_str("修改工作区 ");
        }
        if status.is_wt_deleted() {
            status_str.push_str("删除工作区 ");
        }

        status_str.trim().to_string()
    }
}
