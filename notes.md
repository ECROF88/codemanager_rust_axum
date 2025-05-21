git log --oneline

git log --oneline --graph

git log <file_path> : git log README.md

git log --author="author_name"

It’s recommended to use tower::ServiceBuilder to apply multiple middleware at once, instead of calling layer (or route_layer) repeatedly:


## commit流程
1. 将更改添加至索引，文件更改更新到索引（暂存区）中
2. 将索引写入Tree对象：将当前索引索引文件的快照，将快照作为Tree写入。返回一个Tree对象id（oid）
3. 更新索引文件，将内存中的索引对象状态写回到磁盘上
4. 创建签名，需要提交者的姓名邮箱和时间戳



## sqlx
1. execute：只返回QueryResult
2. fetch_one :  若查询不返回任何行，会返回错误；如果返回多行，只获取第一行
3. fetch_optional：执行查询并返回零或一行结果，返回 Option<Row>
4. fetch_all：执行查询并返回所有行，返回 Vec<Row>


对于枚举类型，需要实现TryFrom trait才能把数据库里面的东西拿出来
也需要derive Type 才能再bind时候传入数据库。


sqlx transaction:
1: let mut tx = pool.begin().await 最后 tx.commit.await
2: 自己添加transaction函数，传入callback
```rust
  pub async fn transaction<F, R, E>(&self, callback: F) -> Result<R, E>
    where
        F: for<'c> FnOnce(&'c mut sqlx::Transaction<'_, sqlx::Postgres>) -> BoxFuture<'c, Result<R, E>>,
        E: From<sqlx::Error>,
    {
        let mut tx = self.pool.begin().await?;
        let result = callback(&mut tx).await?;
        tx.commit().await?;
        Ok(result)
    }
```

超时处理：
```rust 
let result = tokio::time::timeout(
    std::time::Duration::from_secs(5),
    self.pg_db.transaction(|tx| Box::pin(async move { /* ... */ }))
).await??;
```

PostgreSQL 默认的事务隔离级别是 READ COMMITTED。如需更高隔离级别，可以在开始事务时指定
```rust
let mut tx = pool.begin_with_config(
    TransactionOptions::new().isolation(IsolationLevel::Serializable)
).await?;
```