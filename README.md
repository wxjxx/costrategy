# costrategy

公司内部项目任务管理系统。本仓库包含：

- `backend/`：Rust + Actix Web 后端，默认监听 `127.0.0.1:8080`
- `frontend/`：Vue 3 + Vite 前端，开发服务默认监听 `http://localhost:5173`

## 环境准备

本地需要先安装：

- Rust stable toolchain
- Node.js 和 npm
- PostgreSQL
- RustFS 或兼容 S3 的对象存储

仓库根目录的 `local_ENV` 记录了当前本地开发环境的连接信息，可以作为配置来源参考。注意：后端启动时不会自动读取 `local_ENV`，实际会读取当前工作目录下的 `.env` 或系统环境变量。

## 后端配置

在 `backend/` 目录创建 `.env`：

```bash
cd backend
cp ../.env.example .env
```

检查以下变量，并替换为本地真实值：

```env
DATABASE_URL=postgres://<user>:<password>@<host>:<port>/<database>
RUSTFS_ENDPOINT=<host>:<port>
RUSTFS_REGION=<region>
RUSTFS_BUCKET=<bucket>
RUSTFS_ACCESS_KEY_ID=<access-key>
RUSTFS_SECRET_ACCESS_KEY=<secret-key>
```

如需联调钉钉免登、通讯录同步或个人通知，再补充：

```env
DINGTALK_CORP_ID=<corp-id>
DINGTALK_CLIENT_ID=<client-id-or-app-key>
DINGTALK_CLIENT_SECRET=<client-secret-or-app-secret>
DINGTALK_AGENT_ID=<agent-id>
```

`DINGTALK_OAPI_BASE_URL` 可选，默认是 `https://oapi.dingtalk.com`。不配置钉钉变量时，后端仍可本地启动，但钉钉相关接口会返回稳定的配置缺失错误。

当前代码没有读取 Redis 配置；如果 `local_ENV` 中有 Redis 信息，可先保留给后续能力使用。

检查配置和外部依赖：

```bash
cd backend
cargo run --bin config_check
```

执行数据库迁移：

```bash
cd backend
cargo run --bin migrate
```

## 本地启动

启动后端：

```bash
cd backend
cargo run
```

后端默认输出 `info` 级别服务日志和 HTTP 请求日志。需要调整日志级别时可设置 `RUST_LOG`：

```bash
cd backend
RUST_LOG=debug,actix_web=info,actix_server=info cargo run
```

后端启动成功后可以访问：

- 健康检查：`http://127.0.0.1:8080/api/health`
- Swagger UI：`http://127.0.0.1:8080/swagger-ui`
- OpenAPI JSON：`http://127.0.0.1:8080/api-docs/openapi.json`

启动前端：

```bash
cd frontend
npm ci
npm run dev
```

打开 `http://localhost:5173`。开发模式下，Vite 会把 `/api` 请求代理到 `http://127.0.0.1:8080`。

如果在反向代理或已处理 CORS 的环境中运行前端，也可以在 `frontend/.env` 中指定 API 地址：

```env
VITE_API_BASE_URL=http://127.0.0.1:8080/api
```

本地开发建议优先使用默认 `/api` 代理，避免浏览器跨域限制。

## Docker 打包

根目录的 `Dockerfile` 会在一个镜像中完成前端构建、后端 release 构建，并用 nginx 托管前端静态文件、反向代理 `/api` 到同容器内的 Rust 后端。

构建镜像：

```bash
docker build -t costrategy:local .
```

运行镜像：

```bash
docker run --rm -p 8080:80 --env-file backend/.env costrategy:local
```

首次部署或有新迁移时可让容器启动前执行迁移：

```bash
docker run --rm -p 8080:80 --env-file backend/.env -e RUN_MIGRATIONS=true costrategy:local
```

容器对外暴露 `80` 端口；前端访问入口为 `http://127.0.0.1:8080`，API 仍走同源 `/api`。

## 调试

后端常用调试命令：

```bash
cd backend
RUST_BACKTRACE=1 cargo run
curl http://127.0.0.1:8080/api/health
cargo test
```

前端常用调试命令：

```bash
cd frontend
npm run test
npm run test:watch
npm run build
```

排查接口问题时优先确认：

1. `cargo run --bin config_check` 是否通过。
2. `cargo run --bin migrate` 是否已执行。
3. 后端是否运行在 `127.0.0.1:8080`。
4. 前端请求是否走 `/api` 代理，或 `VITE_API_BASE_URL` 是否配置正确。
5. 浏览器 Network 面板里的响应体，后端错误会返回稳定的 `error.code`。

## 补充说明

- 多数业务接口需要登录态 cookie：`costrategy_session`。
- 后端主服务使用运行时钉钉客户端；缺少钉钉配置时不会阻止服务启动，但免登、通讯录同步和个人通知无法真实联调。
- 钉钉配置完整时，后端会启动后台任务：每日 03:00 同步通讯录，09:00 发送截止前一天提醒，09:10 发送延期任务提醒。
- 附件上传依赖 RustFS/S3 桶存在并且访问密钥可用。
