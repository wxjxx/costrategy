# 项目管理系统技术选型

## 1. 选型前提

第一版是钉钉工作台里的企业内部 H5 应用，核心页面是工作台。工作台默认展示看板，并支持切换到甘特图和列表视图。系统还需要钉钉免登、通讯录同步、定向个人通知、富文本任务描述、附件上传和评论区。

本项目按全新项目独立选型，不依赖外部既有项目。

## 2. 总体技术栈

第一版确认使用：

- 前端：Vue 3 + TypeScript + Vite。
- UI 组件库：Element Plus。
- 客户端状态：Pinia。
- 服务端数据请求和缓存：TanStack Query for Vue。
- 路由：Vue Router。
- 看板拖拽：vue.draggable.next。
- 甘特图：DHTMLX Gantt。
- 富文本：Tiptap。
- 后端：Rust + Actix Web + SQLx。
- 数据库：PostgreSQL。
- 文件存储：RustFS。
- 钉钉集成：后端封装钉钉开放平台 HTTP API。

本地开发配置统一放在项目根目录的 `local_ENV` 文件中。该文件包含钉钉、RustFS、数据库等敏感配置，只作为本机环境配置来源，不提交代码仓库。

## 3. 前端选型

### 3.1 Vue 3 + TypeScript + Vite

推荐使用 Vue 3 单文件组件组织前端页面和组件，使用 TypeScript 约束任务、项目、用户、通知等核心数据结构。

理由：

- 系统是企业内部 H5 管理工具，页面以表单、筛选、表格、抽屉、弹窗、看板和甘特图为主。
- Vue 单文件组件适合把工作台、任务详情、任务卡片、筛选区、甘特图任务条等交互拆成独立组件。
- Vite 支持 Vue 和 TypeScript 模板，适合快速开发和构建。

参考：

- Vue SFC: https://vuejs.org/guide/scaling-up/sfc
- Vue + TypeScript: https://vuejs.org/guide/typescript/overview
- Vite: https://vite.dev/guide/

### 3.2 Element Plus

UI 组件库使用 Element Plus。

适用范围：

- 菜单。
- 表单。
- 表格。
- 筛选控件。
- 日期选择。
- 抽屉。
- 弹窗。
- 上传组件。
- Tooltip。
- Tag。
- Button。

理由：

- Element Plus 是 Vue 3 组件库，适合后台管理系统。
- 可以快速完成项目管理、用户管理、系统设置和任务详情等标准业务界面。

参考：

- Element Plus: https://element-plus.org/en-US/

### 3.3 Pinia + TanStack Query

Pinia 用于保存客户端 UI 状态：

- 当前工作台视图：看板、甘特图、列表。
- 当前筛选条件。
- 当前打开的任务详情。
- 当前用户信息和权限。

TanStack Query for Vue 用于管理服务端数据：

- 任务列表。
- 项目列表。
- 用户列表。
- 评论列表。
- 附件列表。
- 通知记录。

理由：

- Pinia 适合本地状态和跨组件共享状态。
- TanStack Query 适合接口请求、缓存、刷新、变更后失效和重新拉取。

参考：

- Pinia: https://pinia.vuejs.org/introduction.html
- TanStack Query Vue: https://tanstack.com/query/latest/docs/framework/vue/overview

## 4. 看板拖拽

看板拖拽使用 `vue.draggable.next`，底层基于 SortableJS。

适用原因：

- 看板需求是多列任务卡片拖拽，属于列表间拖拽场景。
- 支持 Vue 3。
- 支持触摸设备。
- 支持列表间拖拽、拖拽取消和事件回调。

实现约束：

- 拖拽完成后必须调用后端状态变更接口。
- 前端只做交互反馈，后端必须再次校验权限和状态流转规则。
- 已延期列不能作为手动状态列，前端禁止拖入。
- 如果后端拒绝状态变更，前端需要回滚任务卡片位置。

参考：

- vue.draggable.next: https://github.com/SortableJS/vue.draggable.next

## 5. 甘特图选型

甘特图确认使用 DHTMLX Gantt。

### 5.1 使用方式

前端使用 DHTMLX Gantt 原生 JS 组件封装 Vue 3 组件，不使用 DHTMLX Vue Gantt wrapper。

DHTMLX 官方提供 Vue Gantt wrapper，但官方 Quick Start 说明该 wrapper 包含在 Commercial、Enterprise 和 Ultimate editions 中。为避免授权和包访问不确定性，本项目第一版固定采用原生 JS 组件封装 Vue 组件。

封装要求：

- Vue 组件负责容器生命周期，在挂载时初始化 DHTMLX Gantt。
- Vue 组件在卸载时销毁 DHTMLX Gantt 实例和事件监听。
- 工作台筛选条件变化后，由 Vue 组件重新向 DHTMLX Gantt 写入过滤后的任务数据。
- DHTMLX Gantt 的点击、悬停等事件通过 Vue emits 或回调传给工作台页面。

### 5.2 第一版使用能力

第一版只使用 DHTMLX Gantt 的展示能力：

- 根据任务开始日期和截止日期展示任务条。
- 任务条展示任务标题和负责人。
- 任务状态通过任务条颜色展示。
- 鼠标悬停任务条时展示任务卡片内容。
- 点击任务条打开任务详情。
- 支持按天/周切换时间刻度。
- 支持使用工作台统一筛选条件过滤任务。

第一版不启用：

- 甘特图拖拽改期。
- 任务依赖线。
- 任务依赖关系。
- 自动排期。
- 资源负载。
- 关键路径。
- 导出。

### 5.3 授权约束

DHTMLX Gantt 的免费 Standard edition 使用 GPL v2。公司内部闭源业务系统不应在未确认授权的情况下直接使用 GPL 版本。

本项目选择 DHTMLX Gantt 的前提是：

- 项目方确认可合法使用 DHTMLX Gantt。
- 如项目为闭源商业系统，应采购或确认商业授权。
- 授权信息需要在实施前记录到项目文档中。

参考：

- DHTMLX Gantt: https://dhtmlx.com/docs/products/dhtmlxGantt/
- DHTMLX Vue Gantt: https://docs.dhtmlx.com/gantt/integrations/vue/
- DHTMLX Vue Gantt Quick Start: https://docs.dhtmlx.com/gantt/integrations/vue/quick-start/

## 6. 富文本编辑器

任务描述使用 Tiptap。

第一版启用能力：

- 标题。
- 加粗。
- 列表。
- 链接。
- 图片。

存储建议：

- 优先保存 Tiptap JSON，便于后续扩展和安全处理。
- 如需对外展示 HTML，由后端或前端统一渲染并做安全过滤。

安全要求：

- 富文本展示时必须过滤不安全标签和属性。
- 图片上传走系统附件或独立图片上传接口，不允许直接保存不受控外链脚本内容。
- 评论区第一版仍保持纯文本，不接入富文本。

参考：

- Tiptap: https://tiptap.dev/docs/editor/getting-started/overview

## 7. 后端选型

后端确认使用 Rust + Actix Web + SQLx。

### 7.1 Actix Web

Actix Web 用于提供 HTTP API。

适用范围：

- 钉钉免登回调和登录态接口。
- 工作台任务查询接口。
- 任务创建、编辑、状态变更接口。
- 项目管理接口。
- 用户和角色接口。
- 附件上传下载接口。
- 评论接口。
- 通知配置和通知记录接口。

理由：

- Actix Web 适合构建异步 HTTP 服务。
- Rust 类型系统有利于约束权限、任务状态流转和钉钉集成边界。
- 后端运行稳定性和性能余量较好。

参考：

- Actix Web: https://actix.rs/docs/getting-started/

### 7.2 SQLx

数据库访问使用 SQLx。

适用范围：

- PostgreSQL 连接池。
- SQL 查询。
- 事务。
- 数据迁移。
- 任务、项目、用户、附件、评论、通知记录等数据读写。

要求：

- 复杂查询保留 SQL 可读性。
- 核心写操作使用事务。
- 任务状态变更、附件写入、评论写入、通知记录写入需要有清晰的错误处理。

参考：

- SQLx: https://docs.rs/sqlx/latest/sqlx/

## 8. 数据库

数据库确认使用 PostgreSQL。

理由：

- 项目、任务、用户、评论、附件、通知记录都是典型关系数据。
- 富文本描述如果保存为结构化 JSON，可使用 PostgreSQL `jsonb`。
- PostgreSQL `jsonb` 支持 GIN 索引，可在后续需要时优化结构化 JSON 查询。

关键表方向：

- users。
- departments。
- projects。
- tasks。
- task_comments。
- task_attachments。
- task_activity_logs。
- notification_records。
- dingtalk_sync_logs。

参考：

- PostgreSQL JSON/JSONB: https://www.postgresql.org/docs/16/datatype-json.html

## 9. 钉钉集成

第一版使用钉钉开放平台能力：

- 企业内部 H5 应用。
- H5 免登。
- 通讯录管理。
- 企业内部应用个人工作通知。

后端需要封装独立 `DingTalkIntegration` 服务层。

服务层职责：

- 获取和缓存 access token。
- 处理 H5 免登用户身份换取。
- 同步部门。
- 同步用户。
- 发送个人工作通知。
- 记录钉钉接口错误码和请求结果。

参考：

- 钉钉开放平台: https://open.dingtalk.com/document/
- 企业内部 H5 应用: https://open.dingtalk.com/document/development/create-an-h5-application-for-your-enterprise
- 企业内部网页应用免登: https://open.dingtalk.com/document/app/web-app-dingtalk-login-overview-enterprise
- 发送个人工作通知: https://open.dingtalk.com/document/development/send-personal-work-notifications

## 10. 文件存储

文件存储确认使用 RustFS。

RustFS 是 S3 兼容对象存储，后端按 S3 协议接入，用于保存任务附件和富文本图片。

后端需要抽象 `FileStorage` 能力，业务代码不直接依赖 RustFS SDK 或具体存储路径。

附件需要记录：

- 原始文件名。
- 文件大小。
- MIME 类型。
- RustFS bucket。
- 对象 key。
- 上传人。
- 上传时间。

RustFS 配置项包括：

- Endpoint。
- Region。
- Bucket。
- Access Key。
- Secret Key。
- 是否使用 HTTPS。
- 外部访问域名或下载代理地址。

RustFS 的 Access Key 和 Secret Key 必须通过环境变量或密钥管理系统配置，不写入代码仓库。

参考：

- RustFS S3 Compatibility: https://docs.rustfs.com/features/s3-compatibility/
- RustFS SDK Overview: https://docs.rustfs.com/developer/sdk/

## 11. 最终选型结论

最终确认：

- 前端：Vue 3 + TypeScript + Vite。
- UI：Element Plus。
- 状态：Pinia + TanStack Query for Vue。
- 看板拖拽：vue.draggable.next。
- 甘特图：DHTMLX Gantt。
- 富文本：Tiptap。
- 后端：Rust + Actix Web + SQLx。
- 数据库：PostgreSQL。
- 钉钉：后端封装钉钉开放平台 HTTP API。
- 文件：RustFS。

## 12. 实施前需确认

进入实施前，需要确认：

- DHTMLX Gantt 的商业授权或合法使用方式。
- RustFS 的 Endpoint、Region、Bucket、Access Key、Secret Key、外部访问域名或下载代理地址。
- 钉钉企业内部应用的 CorpId、App ID、Client ID、Client Secret、AgentId、应用首页地址、回调域名和权限申请范围。

本地已有配置放在项目根目录 `local_ENV` 文件中；缺失的钉钉参数后续在该文件或部署密钥配置中补全。真实密钥不写入需求文档、技术文档或代码仓库。
