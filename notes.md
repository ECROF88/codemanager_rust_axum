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