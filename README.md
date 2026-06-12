# costrategy

costrategy 是一个面向团队协作的项目任务管理系统，提供项目、任务、人员、通知和系统配置管理能力。项目包含 Rust 后端和 Vue 前端，可本地开发运行，也可打包为单个 Docker 镜像部署。

## 功能特性

- 工作台：支持任务看板、甘特图、列表视图，以及项目、负责人、状态、优先级、日期和关键词筛选。
- 任务管理：支持任务创建、编辑、详情、评论、附件、富文本描述、操作记录和状态流转。
- 多负责人：任务可绑定多个负责人，权限判断、筛选和通知会按多负责人处理。
- 项目管理：支持项目查询、创建、编辑和归档。
- 用户管理：支持钉钉通讯录同步、用户状态和系统角色管理。
- 通知能力：支持任务分配、截止提醒、延期提醒、评论等通知记录。
- 系统设置：支持钉钉、RustFS/S3 对象存储等运行配置展示和维护。
- API 文档：后端内置 OpenAPI JSON 和 Swagger UI。

## 技术栈

- 后端：Rust、Actix Web、SQLx、PostgreSQL、Tokio。
- 前端：Vue 3、Vite、TypeScript、Element Plus、TanStack Query、Pinia、Tiptap。
- 存储：RustFS 或兼容 S3 的对象存储。
- 集成：钉钉免登、通讯录同步和工作通知。
- 部署：Docker 多阶段构建，nginx 托管前端并反向代理后端 API。

## 目录结构

```text
.
├── backend/              # Rust + Actix Web 后端
│   ├── migrations/       # SQLx 数据库迁移
│   └── src/
├── frontend/             # Vue 3 + Vite 前端
│   └── src/
├── docker/               # 单镜像运行所需 nginx 和启动脚本
├── Dockerfile            # 前后端一体化镜像构建
├── .env.example          # 环境变量示例
└── README.md
```

## 环境要求

本地开发需要安装：

- Rust stable toolchain
- Node.js 20+ 和 npm
- PostgreSQL 14+
- RustFS 或兼容 S3 的对象存储

Docker 部署需要安装：

- Docker 24+，或兼容 Dockerfile 的镜像构建工具

## 配置说明

后端启动时会读取当前工作目录下的 `.env` 或系统环境变量。真实密钥不要提交到 Git。

创建后端配置：

```bash
cd /path/to/costrategy
cp .env.example backend/.env
```

至少需要配置：

```env
DATABASE_URL=postgres://<user>:<password>@<host>:<port>/<database>
RUSTFS_ENDPOINT=<host>:<port>
RUSTFS_REGION=<region>
RUSTFS_BUCKET=<bucket>
RUSTFS_ACCESS_KEY_ID=<access-key>
RUSTFS_SECRET_ACCESS_KEY=<secret-key>
```

钉钉集成是可选的。不配置钉钉变量时，后端可以启动，但免登、通讯录同步和真实个人通知不可用。需要联调钉钉时配置：

```env
DINGTALK_CORP_ID=<corp-id>
DINGTALK_CLIENT_ID=<client-id-or-app-key>
DINGTALK_CLIENT_SECRET=<client-secret-or-app-secret>
DINGTALK_AGENT_ID=<agent-id>
DINGTALK_OAPI_BASE_URL=https://oapi.dingtalk.com
```

如需用管理员 token 免登测试，可配置：

```env
ADMIN_AUTH_TOKEN=<random-admin-token>
```

前端开发模式默认通过 Vite 代理访问 `/api`，通常不需要额外配置。只有需要指定 API 地址时，在 `frontend/.env` 中配置：

```env
VITE_API_BASE_URL=http://127.0.0.1:8080/api
VITE_DINGTALK_CLIENT_ID=<client-id>
VITE_DINGTALK_CORP_ID=<corp-id>
```

## 安装与初始化

安装前端依赖：

```bash
cd frontend
npm ci
```

检查后端配置和外部依赖：

```bash
cd backend
cargo run --bin config_check
```

执行数据库迁移：

```bash
cd backend
cargo run --bin migrate
```

## 本地开发

启动后端：

```bash
cd backend
cargo run
```

后端默认监听 `127.0.0.1:8080`。启动后可访问：

- 健康检查：`http://127.0.0.1:8080/api/health`
- Swagger UI：`http://127.0.0.1:8080/swagger-ui`
- OpenAPI JSON：`http://127.0.0.1:8080/api-docs/openapi.json`

启动前端：

```bash
cd frontend
npm run dev
```

前端默认访问地址为 `http://localhost:5173`。开发模式下，Vite 会把 `/api` 代理到 `http://127.0.0.1:8080`。

如果配置了 `ADMIN_AUTH_TOKEN`，可以通过 URL 参数免登：

```text
http://localhost:5173/workbench?admin-token=<random-admin-token>
```

## 构建

构建前端：

```bash
cd frontend
npm run build
```

构建后端 release 二进制：

```bash
cd backend
cargo build --release --locked --bins
```

## Docker 部署

根目录的 `Dockerfile` 会在一个镜像中完成前端构建、后端 release 构建，并用 nginx 托管前端静态文件、反向代理 `/api`、`/api-docs/openapi.json` 和 `/swagger-ui` 到同容器内的 Rust 后端。

构建镜像：

```bash
docker build -t costrategy:local .
```

运行镜像：

```bash
docker run --rm -p 8080:80 --env-file backend/.env costrategy:local
```

首次部署或有新迁移时，可让容器启动前执行迁移：

```bash
docker run --rm -p 8080:80 --env-file backend/.env -e RUN_MIGRATIONS=true costrategy:local
```

容器对外暴露 `80` 端口；前端访问入口为 `http://127.0.0.1:8080`，API 仍走同源 `/api`。

如果容器外还有一层 Nginx、网关或安全代理，需要同时转发 API 文档路径，不能只转发 `/api/`：

```nginx
location ^~ /api/ {
    proxy_pass http://costrategy_container;
}

location = /api-docs/openapi.json {
    proxy_pass http://costrategy_container;
}

location = /swagger-ui {
    proxy_pass http://costrategy_container;
}

location = /swagger-ui/ {
    proxy_pass http://costrategy_container;
}
```

## GitHub Actions 镜像

仓库包含 `.github/workflows/docker-image.yml`，会在以下场景构建 Docker 镜像：

- 推送到 `main` 分支：构建并推送 `latest`、`main` 和 `sha-*` 标签。
- 推送 `v*.*.*` tag：构建并推送对应版本 tag。
- Pull Request：只验证镜像可构建，不推送。
- 手动触发 `workflow_dispatch`：构建并推送镜像。

镜像会发布到 GitHub Container Registry：

```text
ghcr.io/wxjxx/costrategy
```

拉取 latest 镜像：

```bash
docker pull ghcr.io/wxjxx/costrategy:latest
```

运行 GHCR 镜像：

```bash
docker run --rm -p 8080:80 --env-file backend/.env ghcr.io/wxjxx/costrategy:latest
```

每次 workflow 完成后，GitHub Actions 的 job summary 会输出本次打包好的镜像地址和 tag。

## 测试与调试

后端常用命令：

```bash
cd backend
cargo check
cargo test
RUST_BACKTRACE=1 cargo run
RUST_LOG=debug,actix_web=info,actix_server=info cargo run
curl http://127.0.0.1:8080/api/health
```

前端常用命令：

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
6. 后端日志中是否有数据库、钉钉或 RustFS 相关错误。

## 生产部署建议

- 不要把 `.env`、`local_ENV`、密钥、证书、数据库备份提交到仓库。
- 生产环境使用独立 PostgreSQL 和 RustFS/S3 桶，并限制网络访问范围。
- `ADMIN_AUTH_TOKEN` 只用于受控测试或临时管理入口，生产环境请使用高强度随机值，并定期轮换。
- 钉钉 Client Secret、RustFS Secret Key、数据库密码应由环境变量、CI/CD Secret 或密钥管理系统注入。
- 建议在发布前运行 secret 扫描，例如 `gitleaks detect --source . --verbose`。
- 如使用反向代理或 HTTPS 终止，请确保 Cookie、安全头和上传大小限制符合实际部署环境。

## API 文档

后端启动后可访问：

- Swagger UI：`/swagger-ui`
- OpenAPI JSON：`/api-docs/openapi.json`

默认 Docker 部署下完整地址为：

- `http://127.0.0.1:8080/swagger-ui`
- `http://127.0.0.1:8080/api-docs/openapi.json`

## 许可证

本项目基于 [MIT License](LICENSE) 开源。
