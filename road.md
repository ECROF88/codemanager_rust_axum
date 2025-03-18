## frontend 
1. 从后端获取数据，展示代码变更记录
2. 用户可以在前端commit和undo等等操作，使用http请求到后端

## backend
1. 后端要保存代码，包括使用git从github获取代码，提交代码等等
2. 后端对前端的请求进行处理
3. 前端的commit可以自己处理实现，在往github上传的时候再使用git
4. - 实现几个接口：返回代码变更情况，包括增加，删除（+和-，红和绿）
   - 实现对前端请求的处理，commit和undo
   - 实现前端的push请求，使用git向github进行push任亚楠.

   src/
├── main.rs            # 程序入口、全局路由、服务初始化
├── config/            # 项目配置（数据库、JWT 等）
│   └── mod.rs
├── routes/            # 路由模块
│   ├── auth.rs        # 认证相关路由（登录、注册）
│   ├── crud.rs        # 受权限保护的 CRUD 路由
│   └── mod.rs         # 聚合所有子路由
├── middleware/        # 自定义中间件
│   └── auth.rs        # 权限验证中间件
├── handler/           # 请求处理函数（HTTP -> Service 的桥梁）
│   └── mod.rs
├── service/           # 业务逻辑实现（数据库操作、业务规则）
│   └── mod.rs
├── model/             # 数据库模型（直接映射表结构）
│   └── mod.rs
├── dto/               # 数据传输对象（请求/响应结构体）
│   └── mod.rs
├── db/                # 数据库连接池管理
│   └── mod.rs
├── error/             # 自定义错误类型和错误处理
│   └── mod.rs
└── utils/             # 工具类（JWT、密码哈希等）
    └── mod.rs