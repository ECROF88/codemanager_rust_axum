```rust
use config::Config;
use git2::{IndexAddOption, Repository, Signature};
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct GitConfig {
    name: String,
    email: String,
}
#[derive(Debug, Deserialize)]
struct Settings {
    git: GitConfig,
}
fn main() {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();
    let config: Settings = settings.try_deserialize().unwrap();

    println!("config\n {:?}", config);
    let repo = match Repository::init("./testgit") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to init: {}", e),
    };
    let repo = Repository::open("./testgit").unwrap();
    if repo.is_empty().unwrap() {
        println!("Repository is empty");
    } else {
        println!("Repository is not empty");
    }
    
    let mut index = repo.index().unwrap();
    index
        .add_all(["*.rs"].iter(), IndexAddOption::DEFAULT, None)
        .unwrap();

    let status = repo.statuses(None).unwrap();
    if status.is_empty() {
        println!("No change to commit");
    } else {
        println!("{}", status.len());
    }

    // 创建提交的作者和提交者信息
    let signature = Signature::now(&config.git.name, &config.git.email).unwrap();

    // 获取 HEAD 指向的提交 (作为新提交的父提交)
    let head = repo.head().unwrap();
    let parent_commit = head.peel_to_commit().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    // 创建提交
    repo.commit(
        Some("HEAD"),        // 更新 HEAD 指向新的提交
        &signature,          // 作者
        &signature,          // 提交者 (通常与作者相同)
        "My commit message", // 提交信息
        &tree,               // 树对象
        &[&parent_commit],   // 父提交
    )
    .unwrap();

    println!("Commit created successfully");
}

```