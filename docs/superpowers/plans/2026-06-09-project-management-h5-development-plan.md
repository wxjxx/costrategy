# 项目管理 H5 第一版 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付一个运行在钉钉工作台企业内部 H5 中的项目任务管理系统第一版，支持工作台三视图、任务协作、项目/用户/系统设置、钉钉免登/通讯录/个人通知和 RustFS 文件存储。

**Architecture:** 前后端分离，前端用 Vue 3 管理页面状态和原型交互，后端用 Actix Web 提供 REST API、权限校验、钉钉集成、RustFS 存储和定时任务。PostgreSQL 保存业务数据，DHTMLX Gantt 通过原生 JS 封装为 Vue 组件，只启用第一版排期展示能力。

**Tech Stack:** Vue 3、TypeScript、Vite、Element Plus、Pinia、TanStack Query for Vue、vue.draggable.next、DHTMLX Gantt、Tiptap、Rust、Actix Web、SQLx、PostgreSQL、RustFS、钉钉开放平台 HTTP API。

---

## 1. 依据和约束

### 1.1 输入材料

- 需求文档：`docs/superpowers/specs/2026-06-08-project-management-h5-requirements.md`
- 技术选型：`docs/superpowers/specs/2026-06-08-project-management-h5-tech-options.md`
- Pencil 原型：`设计稿/*.pen`
- 原型导出图：`设计稿/exports/*.png`

### 1.2 原型页面映射

| Pencil 文件 | 开发页面/组件 | 实施说明 |
| --- | --- | --- |
| `设计稿/工作台-看板.pen` | `WorkbenchPage`、`TaskKanbanView`、`TaskCard`、统一筛选弹窗 | 系统默认入口；列表和甘特图复用该页面的侧栏、顶部、视图 tab 和筛选交互。 |
| `设计稿/工作台-列表.pen` | `TaskListView` | 复用工作台布局和筛选状态，仅替换中间内容为表格。 |
| `设计稿/工作台-甘特图.pen` | `TaskGanttView`、`DhtmlxGantt` | 复用工作台布局和筛选状态，DHTMLX Gantt 负责时间条。 |
| `设计稿/工作台-任务详情.pen` | `TaskDetailDrawer` 或 `TaskDetailPage` | 展示任务基本信息、富文本描述、附件、评论、操作记录。 |
| `设计稿/工作台-任务新增和编辑.pen` | `TaskFormDrawer`、`RichTextEditor`、附件上传区 | 管理人员创建/编辑任务；员工不展示核心字段编辑入口。 |
| `设计稿/项目管理.pen` | `ProjectsPage`、`ProjectFormDialog` | 只维护项目基础信息，不展示项目下任务。 |
| `设计稿/用户管理.pen` | `UsersPage`、`UserRoleDialog` | 用户来自钉钉通讯录；系统管理员设置角色和停用状态。 |
| `设计稿/系统设置-1.pen` | `SettingsPage` 的钉钉配置、通讯录同步 tab | 管理钉钉配置状态、同步配置和同步日志。 |
| `设计稿/系统设置-2.pen` | `SettingsPage` 的通知规则、通知记录、RustFS 配置 tab | 管理通知规则、查看通知记录、查看或更新 RustFS 配置。 |

### 1.3 第一版不做

- 钉钉审批流、钉钉待办、钉钉 IM 群消息。
- 工时填报、任务依赖、甘特图拖拽改期、甘特图依赖线。
- 评论富文本、附件在线预览、楼中楼评论、@ 提醒。
- 项目管理页查看项目下任务。
- 新评论通知和新附件通知。
- 多级部门数据权限；第一版任务全员可见。

### 1.4 实施前确认项

- DHTMLX Gantt 合法使用方式：闭源内部系统应记录商业授权或其他可合法使用依据。
- 钉钉参数：CorpId、App ID、Client ID、Client Secret、AgentId、H5 首页地址、回调域名、权限申请范围。
- PostgreSQL 和 RustFS 环境已由项目方提供，连接地址和账号密码从根目录 `local_ENV` 读取。
- `local_ENV` 只作为本地敏感配置来源，不提交仓库；开发和测试过程中只校验变量是否存在和连接是否可用，不在日志、文档或错误响应中输出真实值。
- 生产环境使用部署密钥或密钥管理系统维护敏感配置。

## 2. 目标工程结构

```text
costrategy/
  backend/
    Cargo.toml
    migrations/
    src/
      main.rs
      config.rs
      error.rs
      state.rs
      db.rs
      routes.rs
      auth/
      dingtalk/
      users/
      departments/
      projects/
      tasks/
      comments/
      attachments/
      notifications/
      storage/
      settings/
      audit/
    tests/
  frontend/
    package.json
    vite.config.ts
    src/
      main.ts
      App.vue
      router/
      api/
      stores/
      styles/
      layout/
      features/
        workbench/
        tasks/
        projects/
        users/
        settings/
      components/
      test/
  docs/
    api/
    deployment/
  .gitignore
```

## 3. 核心数据模型

### 3.1 PostgreSQL 表

| 表 | 关键字段 | 说明 |
| --- | --- | --- |
| `users` | `id`、`dingtalk_user_id`、`union_id`、`name`、`avatar_url`、`mobile`、`role`、`status`、`last_synced_at` | 钉钉用户映射和系统角色。 |
| `departments` | `id`、`dingtalk_dept_id`、`parent_dingtalk_dept_id`、`name`、`order_no` | 钉钉部门主数据。 |
| `department_users` | `department_id`、`user_id` | 用户和部门多对多关系。 |
| `projects` | `id`、`code`、`name`、`owner_id`、`description`、`start_date`、`end_date`、`status`、`archived_at` | 项目基础信息。 |
| `tasks` | `id`、`project_id`、`title`、`assignee_id`、`status`、`priority`、`start_date`、`due_date`、`description_json`、`creator_id`、`archived_at` | 任务核心数据；延期状态由查询时计算。 |
| `task_comments` | `id`、`task_id`、`author_id`、`content`、`created_at` | 纯文本评论。 |
| `task_attachments` | `id`、`task_id`、`file_name`、`file_size`、`mime_type`、`bucket`、`object_key`、`uploader_id`、`created_at`、`deleted_at` | RustFS 附件索引。 |
| `task_activity_logs` | `id`、`task_id`、`actor_id`、`action`、`before_value`、`after_value`、`created_at` | 任务操作记录。 |
| `notification_rules` | `id`、`rule_type`、`enabled`、`updated_by`、`updated_at` | 通知规则开关。 |
| `notification_records` | `id`、`notification_type`、`receiver_id`、`task_id`、`content_summary`、`status`、`failure_reason`、`sent_at` | 钉钉个人通知发送记录。 |
| `dingtalk_sync_logs` | `id`、`started_at`、`finished_at`、`status`、`created_users`、`updated_users`、`disabled_users`、`failure_reason` | 通讯录同步日志。 |
| `system_settings` | `key`、`value_encrypted`、`value_masked`、`updated_by`、`updated_at` | 保存可由系统设置页维护的敏感配置；加密主密钥来自环境变量。 |

### 3.2 枚举

- `user_role`: `employee`、`manager`、`admin`
- `user_status`: `active`、`disabled`
- `project_status`: `active`、`completed`、`paused`、`archived`
- `task_status`: `todo`、`in_progress`、`done`
- `task_priority`: `low`、`medium`、`high`
- `notification_type`: `task_assigned`、`assignee_changed`、`due_tomorrow`、`task_overdue`

## 4. API 轮廓

### 4.1 鉴权和当前用户

- `POST /api/auth/dingtalk/login`：接收钉钉 H5 免登 code，换取当前用户并建立系统登录态。
- `GET /api/me`：返回当前用户、角色、部门和前端权限点。
- `POST /api/auth/logout`：退出本系统登录态。

### 4.2 工作台和任务

- `GET /api/tasks`：按项目、人员、状态、优先级、日期范围、关键词查询未归档任务。
- `POST /api/tasks`：管理人员和系统管理员创建任务。
- `GET /api/tasks/{task_id}`：读取任务详情、附件、评论、操作记录。
- `PUT /api/tasks/{task_id}`：管理人员和系统管理员编辑任务核心字段。
- `PATCH /api/tasks/{task_id}/status`：变更任务状态；员工只能变更自己负责的任务。
- `POST /api/tasks/{task_id}/archive`：归档任务。
- `POST /api/tasks/{task_id}/comments`：新增纯文本评论。
- `POST /api/tasks/{task_id}/attachments`：上传附件到 RustFS。
- `GET /api/tasks/{task_id}/attachments/{attachment_id}/download`：下载附件。
- `DELETE /api/tasks/{task_id}/attachments/{attachment_id}`：上传人、管理人员、系统管理员删除附件。

### 4.3 项目、用户、设置

- `GET /api/projects`、`POST /api/projects`、`PUT /api/projects/{project_id}`、`POST /api/projects/{project_id}/archive`
- `GET /api/users`、`PATCH /api/users/{user_id}/role`、`PATCH /api/users/{user_id}/status`
- `POST /api/dingtalk/sync`、`GET /api/dingtalk/sync-logs`
- `GET /api/settings`、`PUT /api/settings`
- `GET /api/notification-rules`、`PATCH /api/notification-rules/{rule_type}`
- `GET /api/notification-records`

### 4.4 统一接口错误枚举

所有接口错误统一返回 `ApiErrorCode`，前端只依赖 `error.code` 做稳定提示，`error.message` 作为后端兜底文案。后端日志可以记录内部细节，但接口响应不得返回密钥、连接串、钉钉 access token、RustFS object key 等敏感值。

Error response:

```json
{
  "error": {
    "code": "TASK_INVALID_STATUS_TRANSITION",
    "message": "任务状态流转不允许",
    "details": {
      "task_id": "uuid"
    }
  }
}
```

Error codes:

| Code | HTTP | 前端默认提示 |
| --- | --- | --- |
| `AUTH_NOT_LOGIN` | 401 | 请先登录 |
| `AUTH_FORBIDDEN` | 403 | 当前账号没有操作权限 |
| `AUTH_DINGTALK_LOGIN_FAILED` | 401 | 钉钉免登失败，请重新从钉钉工作台进入 |
| `AUTH_USER_NOT_SYNCED` | 403 | 当前钉钉用户尚未同步到系统 |
| `AUTH_USER_DISABLED` | 403 | 当前账号已停用 |
| `VALIDATION_FAILED` | 400 | 提交内容不符合要求 |
| `RESOURCE_NOT_FOUND` | 404 | 数据不存在或已被删除 |
| `CONFIG_MISSING` | 500 | 系统配置缺失，请联系管理员 |
| `DATABASE_ERROR` | 500 | 数据库操作失败 |
| `INTERNAL_ERROR` | 500 | 系统异常，请稍后重试 |
| `PROJECT_ARCHIVED` | 400 | 项目已归档，不能继续操作 |
| `TASK_INVALID_STATUS_TRANSITION` | 400 | 任务状态流转不允许 |
| `TASK_NOT_ASSIGNEE` | 403 | 只能更新自己负责的任务 |
| `TASK_ASSIGNEE_INACTIVE` | 400 | 负责人账号不可用 |
| `TASK_DATE_RANGE_INVALID` | 400 | 开始日期不能晚于截止日期 |
| `ATTACHMENT_UPLOAD_FAILED` | 500 | 附件上传失败 |
| `ATTACHMENT_DELETE_FORBIDDEN` | 403 | 没有权限删除该附件 |
| `STORAGE_CONFIG_INVALID` | 500 | 文件存储配置不可用 |
| `STORAGE_DOWNLOAD_FAILED` | 500 | 附件下载失败 |
| `DINGTALK_CONFIG_MISSING` | 500 | 钉钉应用配置缺失 |
| `DINGTALK_SYNC_FAILED` | 500 | 钉钉通讯录同步失败 |
| `DINGTALK_NOTIFY_FAILED` | 500 | 钉钉通知发送失败 |

## 5. 迭代计划

### Task 1: 工程骨架和本地运行

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/src/main.ts`
- Create: `frontend/src/App.vue`
- Create: `backend/Cargo.toml`
- Create: `backend/src/main.rs`
- Create: `backend/src/config.rs`
- Modify: `.gitignore`

- [ ] **Step 1: 初始化前端和后端骨架**

Run:

```bash
npm create vite@latest frontend -- --template vue-ts
cd backend && cargo init --bin
```

Expected: `frontend` 能启动 Vite，`backend` 能执行 `cargo run`。

- [ ] **Step 2: 固定基础依赖**

Frontend dependencies:

```bash
cd frontend
npm install element-plus pinia vue-router @tanstack/vue-query axios vue.draggable.next dhtmlx-gantt @tiptap/vue-3 @tiptap/starter-kit @tiptap/extension-link @tiptap/extension-image dompurify
npm install -D vitest @vue/test-utils jsdom typescript vue-tsc eslint prettier
```

Backend dependencies:

```bash
cd backend
cargo add actix-web actix-cors actix-multipart serde serde_json serde_with chrono uuid thiserror anyhow tracing tracing-subscriber dotenvy sqlx tokio reqwest jsonwebtoken aws-sdk-s3 bytes futures-util ammonia
cargo add sqlx --features runtime-tokio-rustls,postgres,uuid,chrono,json,macros,migrate
cargo add tokio --features rt-multi-thread,macros,time
cargo add uuid --features serde,v4
```

- [ ] **Step 3: 校验 `local_ENV` 变量名**

根目录 `local_ENV` 已包含 PostgreSQL 和 RustFS 的连接地址、账号、密码和 Bucket 配置。此步骤只校验必需变量存在，不输出变量值。

```bash
bash -lc '
set -a
. ./local_ENV
set +a
for name in DATABASE_URL RUSTFS_ENDPOINT RUSTFS_REGION RUSTFS_BUCKET RUSTFS_ACCESS_KEY_ID RUSTFS_SECRET_ACCESS_KEY; do
  if [ -z "${!name}" ]; then
    echo "missing $name"
    exit 1
  fi
done
echo "local_ENV variable names ok"
'
```

Expected: 输出 `local_ENV variable names ok`，不输出任何连接串、账号、密码或密钥值。

- [ ] **Step 4: 首次构建验证**

Run:

```bash
cd frontend && npm run build
cd ../backend && cargo test
```

Expected: 前端构建通过，后端测试通过。

Commit:

```bash
git add frontend backend .gitignore
git commit -m "chore: 初始化项目管理系统工程骨架"
```

### Task 2: 后端配置、错误处理和数据库迁移

**Files:**
- Create: `backend/src/config.rs`
- Create: `backend/src/error.rs`
- Create: `backend/src/db.rs`
- Create: `backend/src/bin/config_check.rs`
- Create: `backend/migrations/202606090001_init.sql`
- Create: `frontend/src/api/errorCodes.ts`
- Modify: `backend/src/main.rs`

- [ ] **Step 1: 实现 `local_ENV` 配置加载和连通性检查**

Config keys:

```text
DATABASE_URL
APP_JWT_SECRET
APP_CONFIG_MASTER_KEY
DINGTALK_CORP_ID
DINGTALK_APP_ID
DINGTALK_CLIENT_ID
DINGTALK_CLIENT_SECRET
DINGTALK_AGENT_ID
DINGTALK_CALLBACK_URL
RUSTFS_ENDPOINT
RUSTFS_REGION
RUSTFS_BUCKET
RUSTFS_ACCESS_KEY_ID
RUSTFS_SECRET_ACCESS_KEY
RUSTFS_USE_HTTPS
RUSTFS_PUBLIC_BASE_URL
```

Validation: 后端启动时校验必填项，缺失时输出变量名，不输出变量值。

`backend/src/bin/config_check.rs` reads the same config and verifies PostgreSQL and RustFS connectivity:

```bash
set -a
. ../local_ENV
set +a
cargo run --bin config_check
```

Expected: 输出 `config ok`、`postgres ok`、`rustfs ok`，不输出任何连接串、账号、密码或密钥值。

- [ ] **Step 2: 实现统一接口错误枚举**

`backend/src/error.rs` defines `ApiErrorCode`, `ApiErrorBody`, `ApiErrorResponse` and `AppError`. All routes return errors through `AppError`, and `ResponseError` converts them into the response shape defined in section 4.4.

Backend enum:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorCode {
    AuthNotLogin,
    AuthForbidden,
    AuthDingtalkLoginFailed,
    AuthUserNotSynced,
    AuthUserDisabled,
    ValidationFailed,
    ResourceNotFound,
    ConfigMissing,
    DatabaseError,
    InternalError,
    ProjectArchived,
    TaskInvalidStatusTransition,
    TaskNotAssignee,
    TaskAssigneeInactive,
    TaskDateRangeInvalid,
    AttachmentUploadFailed,
    AttachmentDeleteForbidden,
    StorageConfigInvalid,
    StorageDownloadFailed,
    DingtalkConfigMissing,
    DingtalkSyncFailed,
    DingtalkNotifyFailed,
}
```

Frontend mapping:

```ts
export type ApiErrorCode =
  | 'AUTH_NOT_LOGIN'
  | 'AUTH_FORBIDDEN'
  | 'AUTH_DINGTALK_LOGIN_FAILED'
  | 'AUTH_USER_NOT_SYNCED'
  | 'AUTH_USER_DISABLED'
  | 'VALIDATION_FAILED'
  | 'RESOURCE_NOT_FOUND'
  | 'CONFIG_MISSING'
  | 'DATABASE_ERROR'
  | 'INTERNAL_ERROR'
  | 'PROJECT_ARCHIVED'
  | 'TASK_INVALID_STATUS_TRANSITION'
  | 'TASK_NOT_ASSIGNEE'
  | 'TASK_ASSIGNEE_INACTIVE'
  | 'TASK_DATE_RANGE_INVALID'
  | 'ATTACHMENT_UPLOAD_FAILED'
  | 'ATTACHMENT_DELETE_FORBIDDEN'
  | 'STORAGE_CONFIG_INVALID'
  | 'STORAGE_DOWNLOAD_FAILED'
  | 'DINGTALK_CONFIG_MISSING'
  | 'DINGTALK_SYNC_FAILED'
  | 'DINGTALK_NOTIFY_FAILED';

export const API_ERROR_MESSAGES: Record<ApiErrorCode, string> = {
  AUTH_NOT_LOGIN: '请先登录',
  AUTH_FORBIDDEN: '当前账号没有操作权限',
  AUTH_DINGTALK_LOGIN_FAILED: '钉钉免登失败，请重新从钉钉工作台进入',
  AUTH_USER_NOT_SYNCED: '当前钉钉用户尚未同步到系统',
  AUTH_USER_DISABLED: '当前账号已停用',
  VALIDATION_FAILED: '提交内容不符合要求',
  RESOURCE_NOT_FOUND: '数据不存在或已被删除',
  CONFIG_MISSING: '系统配置缺失，请联系管理员',
  DATABASE_ERROR: '数据库操作失败',
  INTERNAL_ERROR: '系统异常，请稍后重试',
  PROJECT_ARCHIVED: '项目已归档，不能继续操作',
  TASK_INVALID_STATUS_TRANSITION: '任务状态流转不允许',
  TASK_NOT_ASSIGNEE: '只能更新自己负责的任务',
  TASK_ASSIGNEE_INACTIVE: '负责人账号不可用',
  TASK_DATE_RANGE_INVALID: '开始日期不能晚于截止日期',
  ATTACHMENT_UPLOAD_FAILED: '附件上传失败',
  ATTACHMENT_DELETE_FORBIDDEN: '没有权限删除该附件',
  STORAGE_CONFIG_INVALID: '文件存储配置不可用',
  STORAGE_DOWNLOAD_FAILED: '附件下载失败',
  DINGTALK_CONFIG_MISSING: '钉钉应用配置缺失',
  DINGTALK_SYNC_FAILED: '钉钉通讯录同步失败',
  DINGTALK_NOTIFY_FAILED: '钉钉通知发送失败',
};
```

- [ ] **Step 3: 编写初始迁移**

Migration includes tables listed in section 3.1 and enum check constraints listed in section 3.2. Add indexes:

```sql
CREATE INDEX idx_tasks_filters ON tasks (project_id, assignee_id, status, priority, start_date, due_date);
CREATE INDEX idx_tasks_unarchived ON tasks (archived_at) WHERE archived_at IS NULL;
CREATE INDEX idx_comments_task ON task_comments (task_id, created_at);
CREATE INDEX idx_attachments_task ON task_attachments (task_id, created_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_notification_records_time ON notification_records (sent_at DESC);
```

- [ ] **Step 4: 迁移和连接测试**

Run:

```bash
cd backend
cargo run --bin config_check
sqlx migrate run
cargo test
```

Expected: 配置检查通过，数据库迁移成功，统一错误枚举测试通过，后端能创建连接池。

Commit:

```bash
git add backend frontend/src/api/errorCodes.ts
git commit -m "feat: 添加后端配置和数据库基础模型"
```

### Task 3: 钉钉免登、通讯录同步和用户权限

**Files:**
- Create: `backend/src/dingtalk/client.rs`
- Create: `backend/src/dingtalk/token.rs`
- Create: `backend/src/dingtalk/sync.rs`
- Create: `backend/src/auth/routes.rs`
- Create: `backend/src/auth/session.rs`
- Create: `backend/src/users/repository.rs`
- Create: `backend/src/users/routes.rs`
- Create: `backend/tests/dingtalk_auth.rs`
- Create: `frontend/src/api/auth.ts`
- Create: `frontend/src/stores/session.ts`

- [ ] **Step 1: 后端封装钉钉 HTTP 客户端**

Capabilities:

```text
get_access_token()
exchange_login_code(code)
list_departments()
list_users_by_department(department_id)
send_work_notification(user_id, message)
```

Token cache: 按钉钉返回过期时间缓存 access token，提前 5 分钟刷新。

- [ ] **Step 2: 实现 H5 免登接口**

Behavior:

```text
POST /api/auth/dingtalk/login
input: { "code": "钉钉免登 code" }
success: set httpOnly session cookie, return current user
failure: user not synced -> 403, user disabled -> 403
```

- [ ] **Step 3: 实现通讯录同步**

Behavior:

```text
POST /api/dingtalk/sync
allowed: admin
effect: upsert departments, upsert users, refresh department_users, disable users missing from latest sync
log: write dingtalk_sync_logs
```

- [ ] **Step 4: 实现权限中间件**

Rules:

```text
employee: view all unarchived tasks, update own task status, comment, upload, delete own attachment
manager: employee permissions + create/edit/archive tasks + project CRUD + update all task status
admin: manager permissions + user role/status + system settings + sync + notification records
```

- [ ] **Step 5: 测试**

Run:

```bash
cd backend
cargo test dingtalk_auth dingtalk_sync user_permissions
```

Expected: mock 钉钉接口下免登、同步、权限分支全部通过。

Commit:

```bash
git add backend frontend/src/api/auth.ts frontend/src/stores/session.ts
git commit -m "feat: 接入钉钉免登和通讯录同步"
```

### Task 4: 项目管理后端和页面

**Files:**
- Create: `backend/src/projects/model.rs`
- Create: `backend/src/projects/repository.rs`
- Create: `backend/src/projects/routes.rs`
- Create: `frontend/src/features/projects/ProjectsPage.vue`
- Create: `frontend/src/features/projects/ProjectFormDialog.vue`
- Create: `frontend/src/api/projects.ts`
- Create: `frontend/src/types/project.ts`

- [ ] **Step 1: 实现项目 CRUD API**

Rules:

```text
GET /api/projects: all authenticated users
POST /api/projects: manager/admin
PUT /api/projects/{project_id}: manager/admin
POST /api/projects/{project_id}/archive: manager/admin
```

Archived projects do not appear in task creation project selector.

- [ ] **Step 2: 实现项目管理页面**

Use `设计稿/项目管理.pen` as visual source. Page includes filter row, project table, create/edit dialog, archive action. It does not include a task list or project detail task view.

- [ ] **Step 3: 测试**

Run:

```bash
cd backend && cargo test projects
cd ../frontend && npm run test -- ProjectsPage
```

Commit:

```bash
git add backend/src/projects frontend/src/features/projects frontend/src/api/projects.ts frontend/src/types/project.ts
git commit -m "feat: 实现项目管理基础功能"
```

### Task 5: 任务核心 API、筛选和状态流转

**Files:**
- Create: `backend/src/tasks/model.rs`
- Create: `backend/src/tasks/repository.rs`
- Create: `backend/src/tasks/service.rs`
- Create: `backend/src/tasks/routes.rs`
- Create: `backend/src/audit/task_activity.rs`
- Create: `backend/tests/tasks.rs`
- Create: `frontend/src/api/tasks.ts`
- Create: `frontend/src/types/task.ts`

- [ ] **Step 1: 实现任务查询**

Query parameters:

```text
project_id
assignee_id
status
priority
date_from
date_to
keyword
include_archived=false
sort
```

Date filter rule: `start_date <= date_to AND due_date >= date_from`。

Computed field:

```text
is_overdue = status != done AND current_date > due_date
display_status = overdue when is_overdue else status
```

- [ ] **Step 2: 实现创建、编辑、归档任务**

Create/edit fields:

```text
title, project_id, assignee_id, status, priority, start_date, due_date, description_json
```

Validation:

```text
title non-empty
project exists and not archived
assignee active
start_date <= due_date
priority in low/medium/high
status in todo/in_progress/done
```

- [ ] **Step 3: 实现状态流转**

Allowed transitions:

```text
todo -> in_progress
todo -> done
in_progress -> todo
in_progress -> done
done -> in_progress
```

`overdue` is never stored as a task status. Employee can update only assigned tasks; manager/admin can update all tasks.

- [ ] **Step 4: 写操作记录**

Activity actions:

```text
task_created
assignee_changed
status_changed
schedule_changed
priority_changed
task_archived
```

- [ ] **Step 5: 测试**

Run:

```bash
cd backend
cargo test tasks
```

Expected: filter、overdue、permission、transition、activity log tests pass.

Commit:

```bash
git add backend/src/tasks backend/src/audit backend/tests/tasks.rs frontend/src/api/tasks.ts frontend/src/types/task.ts
git commit -m "feat: 实现任务核心接口和状态流转"
```

### Task 6: 前端基础布局、路由和原型样式基线

**Files:**
- Create: `frontend/src/router/index.ts`
- Create: `frontend/src/layout/AppLayout.vue`
- Create: `frontend/src/layout/AppSidebar.vue`
- Create: `frontend/src/layout/AppTopbar.vue`
- Create: `frontend/src/styles/tokens.css`
- Create: `frontend/src/styles/global.css`
- Modify: `frontend/src/App.vue`
- Modify: `frontend/src/main.ts`

- [ ] **Step 1: 建立路由**

Routes:

```text
/workbench
/projects
/users
/settings
fallback -> /workbench
```

- [ ] **Step 2: 还原公共布局**

Use `设计稿/工作台-看板.pen` as sidebar/topbar/style source. Apply same layout to workbench, project management, user management and system settings pages unless the specific Pencil page shows a page-level difference.

- [ ] **Step 3: 权限菜单**

Menu visibility:

```text
工作台: employee/manager/admin
项目管理: manager/admin
用户管理: admin
系统设置: admin
```

- [ ] **Step 4: 测试**

Run:

```bash
cd frontend
npm run build
npm run test -- AppLayout
```

Commit:

```bash
git add frontend/src
git commit -m "feat: 搭建前端布局和路由"
```

### Task 7: 工作台统一筛选和视图状态

**Files:**
- Create: `frontend/src/features/workbench/WorkbenchPage.vue`
- Create: `frontend/src/features/workbench/WorkbenchViewTabs.vue`
- Create: `frontend/src/features/workbench/TaskFilterPopover.vue`
- Create: `frontend/src/stores/workbench.ts`
- Create: `frontend/src/api/users.ts`

- [ ] **Step 1: 实现工作台数据查询状态**

State:

```text
view: kanban | gantt | list
filters: project_id, assignee_id, status, priority, date_range, keyword
```

State behavior: switch view keeps filters unchanged; filter changes invalidate task query.

- [ ] **Step 2: 实现统一筛选弹窗**

Filter fields:

```text
项目、人员、状态、优先级、日期范围、关键词
```

All three views consume the same filtered task list.

- [ ] **Step 3: 测试**

Run:

```bash
cd frontend
npm run test -- WorkbenchPage TaskFilterPopover
```

Commit:

```bash
git add frontend/src/features/workbench frontend/src/stores/workbench.ts frontend/src/api/users.ts
git commit -m "feat: 实现工作台统一筛选"
```

### Task 8: 看板视图和拖拽改状态

**Files:**
- Create: `frontend/src/features/workbench/kanban/TaskKanbanView.vue`
- Create: `frontend/src/features/workbench/kanban/TaskBoardColumn.vue`
- Create: `frontend/src/features/workbench/kanban/TaskCard.vue`

- [ ] **Step 1: 实现看板列**

Columns:

```text
todo -> 待开始
in_progress -> 进行中
done -> 已完成
overdue computed -> 已延期
```

`overdue` column displays delayed tasks but is not a drop target.

- [ ] **Step 2: 实现任务卡片字段**

Fields:

```text
任务标题、所属项目、负责人、截止日期、优先级、是否延期
```

- [ ] **Step 3: 实现拖拽改状态**

Behavior:

```text
on drag end -> call PATCH /api/tasks/{id}/status
success -> invalidate task query
failure -> rollback card position and show error
employee dragging unassigned task -> disable drag handle
```

- [ ] **Step 4: 测试**

Run:

```bash
cd frontend
npm run test -- TaskKanbanView TaskCard
```

Commit:

```bash
git add frontend/src/features/workbench/kanban
git commit -m "feat: 实现工作台看板和拖拽改状态"
```

### Task 9: 列表视图

**Files:**
- Create: `frontend/src/features/workbench/list/TaskListView.vue`

- [ ] **Step 1: 实现列表表格**

Columns:

```text
任务标题、所属项目、负责人、状态、优先级、开始日期、截止日期
```

Sorting:

```text
due_date, priority, status
```

- [ ] **Step 2: 任务详情入口**

Click row opens `TaskDetailDrawer`; manager/admin sees edit action, employee sees view/status/comment/attachment actions.

- [ ] **Step 3: 测试**

Run:

```bash
cd frontend
npm run test -- TaskListView
```

Commit:

```bash
git add frontend/src/features/workbench/list
git commit -m "feat: 实现工作台列表视图"
```

### Task 10: 甘特图视图和 DHTMLX 封装

**Files:**
- Create: `frontend/src/features/workbench/gantt/TaskGanttView.vue`
- Create: `frontend/src/features/workbench/gantt/DhtmlxGantt.vue`
- Create: `frontend/src/features/workbench/gantt/ganttMapping.ts`
- Create: `frontend/src/features/workbench/gantt/ganttStyles.css`

- [ ] **Step 1: 封装 DHTMLX 原生 JS 组件**

Lifecycle:

```text
mounted -> gantt.init(container)
data changed -> gantt.clearAll(); gantt.parse(mappedTasks)
unmounted -> detach all events; gantt.clearAll()
```

- [ ] **Step 2: 实现任务条**

Display:

```text
bar text = 任务标题 + 负责人
bar color = status color
tooltip = task card fields
click bar = open TaskDetailDrawer
```

Status colors:

```text
todo: blue
in_progress: green
done: gray
overdue: red
```

- [ ] **Step 3: 限制第一版交互**

Disable:

```text
drag_move, drag_resize, drag_progress, links, dependency editing
```

Enable:

```text
day/week scale switch
```

- [ ] **Step 4: 测试**

Run:

```bash
cd frontend
npm run test -- DhtmlxGantt TaskGanttView
```

Manual check: switch day/week scale, hover bar shows tooltip, click bar opens detail, filter updates chart.

Commit:

```bash
git add frontend/src/features/workbench/gantt
git commit -m "feat: 实现 DHTMLX 甘特图视图"
```

### Task 11: 任务详情、任务表单和富文本

**Files:**
- Create: `frontend/src/features/tasks/TaskDetailDrawer.vue`
- Create: `frontend/src/features/tasks/TaskFormDrawer.vue`
- Create: `frontend/src/features/tasks/RichTextEditor.vue`
- Create: `frontend/src/features/tasks/TaskActivityTimeline.vue`
- Modify: `backend/src/tasks/routes.rs`
- Modify: `backend/src/tasks/service.rs`

- [ ] **Step 1: 实现任务详情**

Sections:

```text
基本信息、富文本描述、附件列表、评论区、操作记录
```

Every user can view; employee can update own status, comment, upload; manager/admin can edit core fields.

- [ ] **Step 2: 实现任务新增和编辑**

Fields:

```text
任务标题、所属项目、负责人、状态、优先级、开始日期、截止日期、富文本描述
```

Use `设计稿/工作台-任务新增和编辑.pen` as visual source.

- [ ] **Step 3: 实现 Tiptap 富文本**

Enabled extensions:

```text
StarterKit, Link, Image
```

Storage:

```text
description_json: Tiptap JSON
```

Rendering: frontend renders allowed nodes only; any HTML rendering path must sanitize with DOMPurify on frontend or `ammonia` on backend.

- [ ] **Step 4: 测试**

Run:

```bash
cd frontend
npm run test -- TaskDetailDrawer TaskFormDrawer RichTextEditor
cd ../backend
cargo test task_description
```

Commit:

```bash
git add frontend/src/features/tasks backend/src/tasks
git commit -m "feat: 实现任务详情表单和富文本描述"
```

### Task 12: 评论、附件和 RustFS

**Files:**
- Create: `backend/src/comments/routes.rs`
- Create: `backend/src/comments/repository.rs`
- Create: `backend/src/attachments/routes.rs`
- Create: `backend/src/attachments/repository.rs`
- Create: `backend/src/storage/file_storage.rs`
- Create: `backend/src/storage/rustfs_storage.rs`
- Create: `frontend/src/features/tasks/TaskComments.vue`
- Create: `frontend/src/features/tasks/TaskAttachments.vue`
- Create: `frontend/src/api/comments.ts`
- Create: `frontend/src/api/attachments.ts`

- [ ] **Step 1: 实现评论**

Rules:

```text
content plain text, non-empty, max 2000 chars
all authenticated users can comment on unarchived tasks
new comment writes task_activity_logs
new comment does not trigger DingTalk notification
```

- [ ] **Step 2: 实现附件上传和下载**

Rules:

```text
upload -> store object in RustFS -> insert task_attachments -> write task_activity_logs
download -> backend streams RustFS object or redirects to signed URL
delete -> uploader/manager/admin only, soft delete DB row, delete object when storage operation succeeds
new attachment does not trigger DingTalk notification
```

- [ ] **Step 3: 测试**

Run:

```bash
cd backend
cargo test comments attachments storage
cd ../frontend
npm run test -- TaskComments TaskAttachments
```

Commit:

```bash
git add backend/src/comments backend/src/attachments backend/src/storage frontend/src/features/tasks frontend/src/api/comments.ts frontend/src/api/attachments.ts
git commit -m "feat: 实现任务评论和附件上传"
```

### Task 13: 用户管理和系统设置

**Files:**
- Create: `backend/src/settings/routes.rs`
- Create: `backend/src/settings/repository.rs`
- Create: `backend/src/settings/crypto.rs`
- Create: `frontend/src/features/users/UsersPage.vue`
- Create: `frontend/src/features/users/UserRoleDialog.vue`
- Create: `frontend/src/features/settings/SettingsPage.vue`
- Create: `frontend/src/features/settings/DingTalkSettingsTab.vue`
- Create: `frontend/src/features/settings/SyncSettingsTab.vue`
- Create: `frontend/src/features/settings/NotificationRulesTab.vue`
- Create: `frontend/src/features/settings/NotificationRecordsTab.vue`
- Create: `frontend/src/features/settings/RustfsSettingsTab.vue`

- [ ] **Step 1: 实现用户管理**

Actions:

```text
GET /api/users
PATCH /api/users/{user_id}/role
PATCH /api/users/{user_id}/status
```

Only admin can update role/status. User creation is not available in UI.

- [ ] **Step 2: 实现系统设置**

Settings behavior:

```text
read: return masked sensitive values and connection status
write: accept changed sensitive values, encrypt before storing in system_settings
env: local_ENV/deploy secrets override empty DB values
```

Sensitive values are never returned in plaintext after save.

- [ ] **Step 3: 实现通讯录同步和通知记录页面**

Admin actions:

```text
manual sync
view sync logs
toggle notification rules
view notification records
```

- [ ] **Step 4: 测试**

Run:

```bash
cd backend
cargo test users settings
cd ../frontend
npm run test -- UsersPage SettingsPage
```

Commit:

```bash
git add backend/src/settings frontend/src/features/users frontend/src/features/settings
git commit -m "feat: 实现用户管理和系统设置"
```

### Task 14: 钉钉个人通知和定时提醒

**Files:**
- Create: `backend/src/notifications/service.rs`
- Create: `backend/src/notifications/routes.rs`
- Create: `backend/src/notifications/scheduler.rs`
- Modify: `backend/src/tasks/service.rs`
- Modify: `backend/src/main.rs`

- [ ] **Step 1: 实现任务事件通知**

Trigger rules:

```text
new task assigned -> notify assignee
assignee changed -> notify new assignee
comment created -> no notification
attachment uploaded -> no notification
```

- [ ] **Step 2: 实现定时提醒**

Scheduler:

```text
daily 09:00 local time -> due_tomorrow reminders
daily 09:10 local time -> overdue reminders
```

Overdue notification receivers:

```text
task assignee
project owner
```

Deduplication: `(notification_type, task_id, receiver_id, local_date)` only sends once per day.

- [ ] **Step 3: 记录发送结果**

Every send attempt writes `notification_records` with `success` or `failed` and failure reason.

- [ ] **Step 4: 测试**

Run:

```bash
cd backend
cargo test notifications
```

Expected: notification triggers, no-comment/no-attachment notifications, schedule deduplication all pass.

Commit:

```bash
git add backend/src/notifications backend/src/tasks backend/src/main.rs
git commit -m "feat: 实现钉钉个人通知和定时提醒"
```

### Task 15: 端到端联调、权限验收和部署文档

**Files:**
- Create: `frontend/playwright.config.ts`
- Create: `frontend/e2e/workbench.spec.ts`
- Create: `frontend/e2e/task-detail.spec.ts`
- Create: `docs/api/rest-api.md`
- Create: `docs/deployment/local-development.md`
- Create: `docs/deployment/dingtalk-h5.md`

- [ ] **Step 1: 写端到端用例**

Scenarios:

```text
login as employee -> see workbench kanban -> filter tasks -> open detail -> comment -> upload attachment
login as employee -> cannot create task -> can drag own task -> cannot drag others task
login as manager -> create task -> edit assignee -> drag any task -> list/gantt share filters
login as admin -> sync contacts -> update user role -> view settings and notification records
```

- [ ] **Step 2: 执行全量验证**

Run:

```bash
cd backend && cargo test
cd ../frontend && npm run build && npm run test && npx playwright test
```

Expected: all unit, integration and e2e tests pass.

- [ ] **Step 3: 编写部署说明**

Docs include:

```text
local_ENV variable list without secret values
database migration command
PostgreSQL connection verification command
RustFS connection and bucket verification command
DingTalk H5 app homepage/callback configuration
DHTMLX Gantt authorization record location
```

Commit:

```bash
git add frontend/e2e frontend/playwright.config.ts docs/api docs/deployment
git commit -m "test: 添加端到端验收和部署文档"
```

## 6. 开发执行顺序

| 顺序 | 目标 | 完成条件 |
| --- | --- | --- |
| 1 | 工程骨架、配置加载、统一错误枚举 | 前后端可构建；`local_ENV` 可加载；PostgreSQL 和 RustFS 连通性检查通过；错误 code 前后端一致。 |
| 2 | 数据库迁移、钉钉免登、通讯录同步、权限基础 | 迁移可执行；钉钉 mock 测试通过；用户角色和权限中间件生效。 |
| 3 | 项目和任务核心 API | 项目 CRUD、任务查询、创建、编辑、归档、状态流转、操作记录测试通过。 |
| 4 | 前端布局和工作台三视图 | 工作台默认看板；列表和甘特图复用看板布局、tab、筛选弹窗；筛选状态跨视图保持。 |
| 5 | 任务详情、富文本、评论、附件 | Tiptap 描述保存和安全展示；RustFS 上传下载删除；评论和操作记录完整。 |
| 6 | 用户管理、系统设置、钉钉通知 | 用户角色/状态、配置、同步日志、通知规则、通知记录、个人通知发送链路完成。 |
| 7 | 端到端联调和验收 | 钉钉工作台入口、权限、三视图、任务协作、通知、附件和设置流程通过验收清单。 |

## 7. 验收清单

- [ ] 用户可以从钉钉工作台打开 H5 并免登进入。
- [ ] 未同步或已停用用户不能进入系统。
- [ ] 进入系统默认打开工作台看板视图。
- [ ] 全员可以查看全部未归档任务。
- [ ] 工作台按项目、人员、状态、优先级、日期范围、关键词筛选，三种视图筛选条件保持一致。
- [ ] 看板任务卡片只展示标题、项目、负责人、截止日期、优先级、是否延期。
- [ ] 员工只能拖拽自己负责的任务；管理人员和系统管理员可以拖拽全部任务。
- [ ] 已延期是展示状态，不作为数据库任务状态保存。
- [ ] 甘特图任务条展示任务标题和负责人，状态通过颜色体现，悬停展示任务卡片内容。
- [ ] 甘特图支持天/周刻度切换，点击任务条打开任务详情，不支持拖拽改期和依赖线。
- [ ] 列表视图支持截止日期、优先级、状态排序。
- [ ] 管理人员和系统管理员可以创建、编辑、分配、归档任务。
- [ ] 任务描述支持 Tiptap 富文本，保存为 JSON，展示时过滤不安全内容。
- [ ] 任务附件上传到 RustFS，支持下载和权限删除，不支持在线预览。
- [ ] 评论为纯文本，新增评论不触发钉钉通知。
- [ ] 新附件不触发钉钉通知。
- [ ] 任务详情展示创建、负责人变更、状态变更、排期变更、优先级变更、附件、评论等操作记录。
- [ ] 项目管理只维护项目基础信息，不展示项目下任务。
- [ ] 用户管理只能由系统管理员设置角色和启停状态。
- [ ] 系统设置能执行通讯录同步、查看同步日志、管理通知规则、查看通知记录、查看或更新加密配置。
- [ ] 新任务分配、负责人变更、截止前一天、任务延期能发送钉钉个人通知并记录结果。

## 8. 覆盖复核

- 工作台：Task 7、Task 8、Task 9、Task 10 覆盖看板、列表、甘特图、筛选和视图复用。
- 任务协作：Task 5、Task 11、Task 12 覆盖任务核心、详情、富文本、评论、附件和操作记录。
- 项目管理：Task 4 覆盖项目基础维护，并明确不展示项目下任务。
- 用户管理：Task 3、Task 13 覆盖钉钉通讯录用户、角色、启停状态。
- 系统设置：Task 13、Task 14 覆盖钉钉配置、同步、RustFS、通知规则和通知记录。
- 钉钉集成：Task 3、Task 14、Task 15 覆盖免登、通讯录、个人通知和联调文档。
- 技术选型：Task 1 到 Task 15 覆盖 Vue 3、Element Plus、Pinia、TanStack Query、vue.draggable.next、DHTMLX Gantt、Tiptap、Rust、Actix Web、SQLx、PostgreSQL、RustFS。
