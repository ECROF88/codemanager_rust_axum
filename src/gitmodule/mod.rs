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
use structs::{CommitDetail, CommitFileChange, CommitInfo, GitFileEntry};

pub mod structs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitConfig {
    pub name: String,
    pub email: String,
}

// #[derive(Debug, Serialize)]
// pub struct CommitInfo {
//     pub id: String,
//     pub author: String,
//     pub message: String,
//     pub time: i64,
// }

// #[derive(Debug, Serialize)]
// pub struct GitOperationResult {
//     pub success: bool,
//     pub message: String,
//     pub data: Option<serde_json::Value>,
// }

// #[derive(Debug, Serialize)]
// pub struct GitFileEntry {
//     pub name: String,
//     pub path: String,
//     pub is_dir: bool,
//     pub size: Option<u64>, // 仅对文件有效
// }

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
        let r = tokio::task::spawn_blocking(async move || {
            RepoBuilder::new()
                .clone(&repo_url, &repo_path_clone)
                .map_err(|e| e)
                .map(|_| repo_path_clone.to_string_lossy().to_string())
        })
        .await
        .map_err(|e| AppError::InternalServerError(format!("Thread panicked: {:?}", e)))?
        .await
        .map_err(|e| AppError::InternalServerError(format!("Clone failed: {}", e)))?; // 等待线程完成并解包结果

        Ok(r)
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

    pub fn get_commit_histories(
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

    // pub fn get_repo_status(&self, repo_name: &str) -> Result<Vec<String>, AppError> {
    //     let repo_path = self.base_path.join(repo_name);
    //     let repo = self.open_repo(&repo_path)?;

    //     let mut status = Vec::new();
    //     let statuses = repo
    //         .statuses(None)
    //         .map_err(|e| AppError::InternalServerError(format!("Failed to get status: {}", e)))?;

    //     for entry in statuses.iter() {
    //         let path = entry.path().unwrap_or("");
    //         let status_str = format!("{}: {}", path, self.status_to_string(entry.status()));
    //         status.push(status_str);
    //     }

    //     Ok(status)
    // }

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

    pub fn list_repository_files(
        &self,
        user_id: &str,
        repo_name: &str,
        directory_path: Option<&str>,
        branch: Option<&str>,
    ) -> Result<Vec<GitFileEntry>, AppError> {
        let repo_path = self.get_user_repo_path(user_id, repo_name);
        let repo = self.open_repo(&repo_path)?;

        // 获取指定分支或默认分支的引用
        let reference = match branch {
            Some(branch_name) => repo
                .find_branch(branch_name, git2::BranchType::Local)
                .map_err(|_| AppError::NotFound(format!("Branch not found: {}", branch_name)))?
                .into_reference(),
            None => repo
                .head()
                .map_err(|e| AppError::InternalServerError(format!("Failed to get HEAD: {}", e)))?,
        };

        // 获取引用指向的提交
        let commit = reference
            .peel_to_commit()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get commit: {}", e)))?;

        // 获取提交的树
        let tree = commit
            .tree()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get tree: {}", e)))?;

        // 如果指定了目录路径，获取该目录的树
        let dir_tree = if let Some(path) = directory_path {
            if path.is_empty() {
                tree
            } else {
                let entry = tree
                    .get_path(Path::new(path))
                    .map_err(|_| AppError::NotFound(format!("Directory not found: {}", path)))?;

                entry
                    .to_object(&repo)
                    .map_err(|e| {
                        AppError::InternalServerError(format!("Failed to get object: {}", e))
                    })?
                    .as_tree()
                    .ok_or(AppError::BadRequest("Path is not a directory".to_string()))?
                    .clone()
            }
        } else {
            tree
        };

        let mut files = Vec::new();

        // 遍历树中的条目
        for entry in dir_tree.iter() {
            let entry_name = entry.name().unwrap_or_default().to_string();
            let entry_path = match directory_path {
                Some(path) if !path.is_empty() => format!("{}/{}", path, entry_name),
                _ => entry_name.clone(),
            };

            let object = entry.to_object(&repo).map_err(|e| {
                AppError::InternalServerError(format!("Failed to get object: {}", e))
            })?;

            let is_dir = object.as_tree().is_some();
            let size = if !is_dir {
                object.as_blob().map(|b| b.content().len() as u64)
            } else {
                None
            };

            files.push(GitFileEntry {
                name: entry_name,
                path: entry_path,
                is_dir,
                size,
            });
        }

        Ok(files)
    }

    pub fn get_file_content(
        &self,
        user_id: &str,
        repo_name: &str,
        file_path: &str,
        branch: Option<&str>,
    ) -> Result<String, AppError> {
        let repo_path = self.get_user_repo_path(user_id, repo_name);
        let repo = self.open_repo(&repo_path)?;

        // 获取指定分支或默认分支的引用
        let reference = match branch {
            Some(branch_name) => repo
                .find_branch(branch_name, git2::BranchType::Local)
                .map_err(|e| AppError::NotFound(format!("Branch not found: {}", e)))?
                .into_reference(),
            None => repo
                .head()
                .map_err(|e| AppError::InternalServerError(format!("Failed to get HEAD: {}", e)))?,
        };

        // 获取引用指向的提交
        let commit = reference
            .peel_to_commit()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get commit: {}", e)))?;

        // 获取提交的树
        let tree = commit
            .tree()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get tree: {}", e)))?;

        // 在树中查找文件
        let entry = tree
            .get_path(Path::new(file_path))
            .map_err(|_| AppError::NotFound(format!("File not found: {}", file_path)))?;

        // 获取文件对象
        let object = entry
            .to_object(&repo)
            .map_err(|e| AppError::InternalServerError(format!("Failed to get object: {}", e)))?;

        // 获取文件 blob
        let blob = object.as_blob().ok_or(AppError::InternalServerError(
            "Object is not a blob".to_string(),
        ))?;

        // 获取文件内容
        let content = std::str::from_utf8(blob.content())
            .map_err(|_| {
                AppError::InternalServerError("File content is not valid UTF-8".to_string())
            })?
            .to_string();

        Ok(content)
    }

    pub fn update_file(
        &self,
        user_id: &str,
        repo_name: &str,
        file_path: &str,
        content: &str,
        commit_message: &str,
        user_email: &str,
    ) -> Result<String, AppError> {
        let repo_path = self.get_user_repo_path(user_id, repo_name);
        let repo = self.open_repo(&repo_path)?;
        let full_file_path = repo_path.join(file_path);

        // 确保目录存在
        if let Some(parent) = full_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create directory: {}", e))
            })?;
        }

        // 写入文件内容
        std::fs::write(&full_file_path, content)
            .map_err(|e| AppError::InternalServerError(format!("Failed to write file: {}", e)))?;

        // 获取索引
        let mut index = repo
            .index()
            .map_err(|e| AppError::InternalServerError(format!("Failed to get index: {}", e)))?;

        // 添加文件到索引
        index.add_path(Path::new(file_path)).map_err(|e| {
            AppError::InternalServerError(format!("Failed to add file to index: {}", e))
        })?;

        // 写入索引
        let oid = index
            .write_tree()
            .map_err(|e| AppError::InternalServerError(format!("Failed to write index: {}", e)))?;

        // 写入索引文件
        index.write().map_err(|e| {
            AppError::InternalServerError(format!("Failed to write index file: {}", e))
        })?;

        // 创建签名
        let signature = Signature::now(user_id, user_email).map_err(|e| {
            AppError::InternalServerError(format!("Failed to create signature: {}", e))
        })?;

        // 查找树
        let tree = repo
            .find_tree(oid)
            .map_err(|e| AppError::InternalServerError(format!("Failed to find tree: {}", e)))?;

        // 获取父提交
        let parent_commit = match repo.head() {
            Ok(head) => match head.peel_to_commit() {
                Ok(commit) => Some(commit),
                Err(e) => {
                    return Err(AppError::InternalServerError(format!(
                        "Failed to peel HEAD: {}",
                        e
                    )));
                }
            },
            Err(e)
                if e.code() == git2::ErrorCode::UnbornBranch
                    || e.code() == git2::ErrorCode::NotFound =>
            {
                None
            }
            Err(e) => {
                return Err(AppError::InternalServerError(format!(
                    "Failed to get HEAD: {}",
                    e
                )));
            }
        };

        // 准备父提交引用
        let parents_vec: Vec<&git2::Commit> =
            parent_commit.as_ref().map_or(Vec::new(), |c| vec![c]);

        // 创建提交
        let commit_id = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                commit_message,
                &tree,
                &parents_vec,
            )
            .map_err(|e| AppError::InternalServerError(format!("Failed to commit: {}", e)))?;

        println!("commit success {}", commit_id);
        Ok(commit_id.to_string())
    }

    pub async fn get_commit_detail(
        &self,
        user_id: &str,
        repo_name: &str,
        commit_id: &str,
    ) -> Result<CommitDetail, AppError> {
        let repo_path = self.get_user_repo_path(user_id, repo_name);
        let repo = self.open_repo(&repo_path)?;

        // 解析提交 ID
        let oid = git2::Oid::from_str(commit_id)
            .map_err(|e| AppError::BadRequest(format!("Invalid commit ID: {}", e)))?;

        // 获取指定的提交
        let commit = repo
            .find_commit(oid)
            .map_err(|e| AppError::NotFound(format!("Commit not found: {}", e)))?;

        // 构建基本提交信息
        let commit_info: CommitInfo = CommitInfo {
            id: commit.id().to_string(),
            author: format!(
                "{} <{}>",
                commit.author().name().unwrap_or(""),
                commit.author().email().unwrap_or("")
            ),
            message: commit.message().unwrap_or("").to_string(),
            time: commit.time().seconds(),
        };

        // 获取父提交
        let parent_commit = if commit.parent_count() > 0 {
            commit.parent(0).ok()
        } else {
            None
        };

        // 获取提交树
        let commit_tree = commit.tree().map_err(|e| {
            AppError::InternalServerError(format!("Failed to get commit tree: {}", e))
        })?;

        let mut file_changes = Vec::new();

        // 如果有父提交，比较与父提交的差异
        if let Some(parent) = parent_commit {
            let parent_tree = parent.tree().map_err(|e| {
                AppError::InternalServerError(format!("Failed to get parent tree: {}", e))
            })?;

            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)
                .map_err(|e| {
                    AppError::InternalServerError(format!("Failed to compute diff: {}", e))
                })?;

            // 处理每个修改的文件
            self.process_diff_into_changes(&diff, &mut file_changes)?;
        } else {
            // 第一次提交，与空树比较
            let empty_tree = {
                let treebuilder = repo.treebuilder(None).map_err(|e| {
                    AppError::InternalServerError(format!("Failed to create treebuilder: {}", e))
                })?;
                let oid = treebuilder.write().map_err(|e| {
                    AppError::InternalServerError(format!("Failed to create empty tree: {}", e))
                })?;
                repo.find_tree(oid).map_err(|e| {
                    AppError::InternalServerError(format!("Failed to find empty tree: {}", e))
                })?
            };

            let diff = repo
                .diff_tree_to_tree(Some(&empty_tree), Some(&commit_tree), None)
                .map_err(|e| {
                    AppError::InternalServerError(format!("Failed to compute diff: {}", e))
                })?;

            // 处理每个修改的文件
            // self.process_diff_into_changes(&repo, &diff, &mut file_changes)?;
            self.process_diff_into_changes(&diff, &mut file_changes)?;
        }

        Ok(CommitDetail {
            commit_info,
            file_changes,
        })
    }

    fn process_diff_into_changes(
        &self,
        // _repo: &Repository,
        diff: &git2::Diff,
        file_changes: &mut Vec<CommitFileChange>,
    ) -> Result<(), AppError> {
        // 首先收集所有修改的文件信息
        diff.foreach(
            &mut |delta, _| {
                let status = match delta.status() {
                    git2::Delta::Added => "added".to_string(),
                    git2::Delta::Deleted => "deleted".to_string(),
                    git2::Delta::Modified => "modified".to_string(),
                    git2::Delta::Renamed => "renamed".to_string(),
                    git2::Delta::Copied => "copied".to_string(),
                    _ => "changed".to_string(),
                };

                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let new_path = delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let path = if delta.status() == git2::Delta::Deleted {
                    old_path
                } else {
                    new_path
                };

                file_changes.push(CommitFileChange {
                    path,
                    status,
                    diff: None, // 将在后续步骤填充
                });

                true
            },
            None,
            None,
            None,
        )
        .map_err(|e| AppError::InternalServerError(format!("Failed to process diff: {}", e)))?;

        // 然后为每个文件获取详细的差异
        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            // 根据delta找到对应的文件变更记录
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            // 查找对应的文件变更记录
            if let Some(pos) = file_changes.iter().position(|change| change.path == path) {
                let file_change = &mut file_changes[pos];

                // if not init:
                if file_change.diff.is_none() {
                    file_change.diff = Some(String::new());
                }

                // 添加行内容到差异文本
                if let Some(diff_text) = &mut file_change.diff {
                    let prefix = match line.origin() {
                        '+' => "+",
                        '-' => "-",
                        'H' => "",
                        _ => " ",
                    };

                    let content = std::str::from_utf8(line.content()).unwrap_or("[Invalid UTF-8]");
                    diff_text.push_str(&format!("{}{}", prefix, content));
                }
            }

            true
        })
        .map_err(|e| AppError::InternalServerError(format!("Failed to print diff: {}", e)))?;

        Ok(())
    }
    // 辅助方法：状态码转字符串
    // fn status_to_string(&self, status: git2::Status) -> String {
    //     let mut status_str = String::new();

    //     if status.is_index_new() {
    //         status_str.push_str("新增索引 ");
    //     }
    //     if status.is_index_modified() {
    //         status_str.push_str("修改索引 ");
    //     }
    //     if status.is_index_deleted() {
    //         status_str.push_str("删除索引 ");
    //     }
    //     if status.is_wt_new() {
    //         status_str.push_str("新增工作区 ");
    //     }
    //     if status.is_wt_modified() {
    //         status_str.push_str("修改工作区 ");
    //     }
    //     if status.is_wt_deleted() {
    //         status_str.push_str("删除工作区 ");
    //     }

    //     status_str.trim().to_string()
    // }
}
