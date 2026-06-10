use actix_web::{web, HttpResponse};
use serde_json::{json, Value};

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .route("/api-docs/openapi.json", web::get().to(openapi_json))
        .route("/swagger-ui", web::get().to(swagger_ui))
        .route("/swagger-ui/", web::get().to(swagger_ui));
}

async fn openapi_json() -> HttpResponse {
    HttpResponse::Ok().json(openapi_spec())
}

async fn swagger_ui() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(SWAGGER_UI_HTML)
}

fn openapi_spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "项目管理系统 API",
            "version": "0.1.0",
            "description": "公司内部项目任务管理系统第一版后端接口文档"
        },
        "servers": [
            { "url": "/api", "description": "当前服务 API 前缀" }
        ],
        "tags": [
            { "name": "Health", "description": "健康检查" },
            { "name": "Auth", "description": "钉钉免登和当前用户" },
            { "name": "DingTalk", "description": "钉钉通讯录同步" },
            { "name": "Projects", "description": "项目管理" },
            { "name": "Users", "description": "用户管理" },
            { "name": "Settings", "description": "系统设置" },
            { "name": "Tasks", "description": "任务工作台和任务详情" },
            { "name": "Attachments", "description": "任务附件" },
            { "name": "Notifications", "description": "通知记录" }
        ],
        "paths": paths(),
        "components": components()
    })
}

fn paths() -> Value {
    json!({
        "/api/health": {
            "get": operation("Health", "健康检查", "返回服务健康状态", Value::Null, response_ref("HealthResponse"))
        },
        "/api/auth/dingtalk/login": {
            "post": operation(
                "Auth",
                "钉钉 H5 免登",
                "接收钉钉免登 code，换取系统登录态并写入 session cookie",
                json_ref_body("DingtalkLoginRequest"),
                response_ref("CurrentUser")
            )
        },
        "/api/auth/admin-token/login": {
            "post": operation(
                "Auth",
                "管理员授权 token 登录",
                "接收后端配置的管理员授权 token，换取系统管理员登录态并写入 session cookie",
                json_ref_body("AdminTokenLoginRequest"),
                response_ref("CurrentUser")
            )
        },
        "/api/auth/logout": {
            "post": secured_operation(
                "Auth",
                "退出登录",
                "退出本系统登录态并清理 session cookie",
                Value::Null,
                empty_response()
            )
        },
        "/api/me": {
            "get": secured_operation("Auth", "获取当前用户", "返回当前登录用户、角色和权限点", Value::Null, response_ref("CurrentUser"))
        },
        "/api/dingtalk/sync": {
            "post": secured_operation("DingTalk", "同步钉钉通讯录", "系统管理员手动同步钉钉部门和用户", Value::Null, response_ref("DingtalkSyncResult"))
        },
        "/api/dingtalk/sync-logs": {
            "get": secured_operation("DingTalk", "通讯录同步日志", "系统管理员查看钉钉通讯录同步结果", Value::Null, array_response_ref("DingtalkSyncLog"))
        },
        "/api/projects": {
            "get": secured_operation("Projects", "项目列表", "查询未归档项目", Value::Null, array_response_ref("Project")),
            "post": secured_operation("Projects", "创建项目", "管理人员或系统管理员创建项目", json_ref_body("ProjectPayload"), response_ref("Project"))
        },
        "/api/projects/{project_id}": {
            "put": secured_operation_with_params(
                "Projects",
                "编辑项目",
                "更新项目基础信息",
                vec![path_uuid_param("project_id", "项目 ID")],
                json_ref_body("ProjectPayload"),
                response_ref("Project")
            )
        },
        "/api/projects/{project_id}/archive": {
            "post": secured_operation_with_params(
                "Projects",
                "归档项目",
                "归档项目基础信息，不包含项目下任务视图",
                vec![path_uuid_param("project_id", "项目 ID")],
                Value::Null,
                response_ref("Project")
            )
        },
        "/api/users": {
            "get": secured_operation("Users", "用户列表", "系统管理员查看钉钉同步用户和系统角色状态", Value::Null, array_response_ref("User"))
        },
        "/api/users/{user_id}/role": {
            "patch": secured_operation_with_params(
                "Users",
                "设置用户角色",
                "系统管理员设置用户系统角色",
                vec![path_uuid_param("user_id", "用户 ID")],
                json_ref_body("UpdateUserRoleRequest"),
                response_ref("User")
            )
        },
        "/api/users/{user_id}/status": {
            "patch": secured_operation_with_params(
                "Users",
                "设置用户状态",
                "系统管理员启用或停用用户",
                vec![path_uuid_param("user_id", "用户 ID")],
                json_ref_body("UpdateUserStatusRequest"),
                response_ref("User")
            )
        },
        "/api/settings": {
            "get": secured_operation("Settings", "系统设置", "系统管理员读取钉钉和 RustFS 配置脱敏状态", Value::Null, response_ref("SettingsResponse")),
            "put": secured_operation("Settings", "保存系统设置", "系统管理员保存系统配置，敏感值不返回明文", json_ref_body("SettingsUpdateRequest"), response_ref("SettingsResponse"))
        },
        "/api/tasks": {
            "get": secured_operation_with_params(
                "Tasks",
                "任务列表",
                "按项目、人员、状态、优先级、日期范围和关键词查询任务，默认不返回归档任务",
                vec![
                    query_param("keyword", "string", "任务标题关键词"),
                    query_param("project_id", "string", "项目 ID"),
                    query_param("assignee_id", "string", "负责人 ID"),
                    query_param("status", "string", "任务状态：todo、in_progress、done"),
                    query_param("priority", "string", "优先级：low、medium、high"),
                    query_param("date_from", "string", "开始日期，YYYY-MM-DD"),
                    query_param("date_to", "string", "截止日期，YYYY-MM-DD"),
                    query_param("include_archived", "boolean", "是否包含已归档任务，默认 false"),
                    query_param("sort", "string", "排序字段：due_date、priority、status")
                ],
                Value::Null,
                array_response_ref("Task")
            ),
            "post": secured_operation("Tasks", "创建任务", "管理人员或系统管理员创建任务并触发负责人通知", json_ref_body("TaskPayload"), response_ref("Task"))
        },
        "/api/tasks/{task_id}": {
            "get": secured_operation_with_params(
                "Tasks",
                "任务详情",
                "读取任务、附件、评论和操作记录",
                vec![path_uuid_param("task_id", "任务 ID")],
                Value::Null,
                response_ref("TaskDetail")
            ),
            "put": secured_operation_with_params(
                "Tasks",
                "编辑任务",
                "管理人员或系统管理员编辑任务核心字段",
                vec![path_uuid_param("task_id", "任务 ID")],
                json_ref_body("TaskPayload"),
                response_ref("Task")
            )
        },
        "/api/tasks/{task_id}/status": {
            "patch": secured_operation_with_params(
                "Tasks",
                "更新任务状态",
                "员工只能更新自己负责的任务状态，管理人员可更新全部任务状态",
                vec![path_uuid_param("task_id", "任务 ID")],
                json_ref_body("UpdateTaskStatusRequest"),
                response_ref("Task")
            )
        },
        "/api/tasks/{task_id}/archive": {
            "post": secured_operation_with_params(
                "Tasks",
                "归档任务",
                "管理人员或系统管理员归档任务",
                vec![path_uuid_param("task_id", "任务 ID")],
                Value::Null,
                response_ref("Task")
            )
        },
        "/api/tasks/{task_id}/comments": {
            "post": secured_operation_with_params(
                "Tasks",
                "新增评论",
                "新增纯文本评论；第一版不触发钉钉通知",
                vec![path_uuid_param("task_id", "任务 ID")],
                json_ref_body("CreateTaskCommentRequest"),
                response_ref("TaskComment")
            )
        },
        "/api/rich-text/images": {
            "post": secured_operation(
                "Attachments",
                "上传富文本图片",
                "上传富文本编辑器中的本地或粘贴图片到 RustFS，并返回可用于描述 JSON 的后端图片 URL",
                multipart_file_body(),
                response_ref("RichTextImageUploadResponse")
            )
        },
        "/api/rich-text/images/{file_name}": {
            "get": secured_operation_with_params(
                "Attachments",
                "读取富文本图片",
                "通过后端鉴权代理读取 RustFS 中的富文本图片",
                vec![path_param("file_name", "string", "图片文件名")],
                Value::Null,
                binary_response()
            )
        },
        "/api/tasks/{task_id}/attachments": {
            "post": secured_operation_with_params(
                "Attachments",
                "上传附件",
                "上传任务附件到 RustFS；第一版不触发钉钉通知",
                vec![path_uuid_param("task_id", "任务 ID")],
                multipart_file_body(),
                response_ref("TaskAttachment")
            )
        },
        "/api/tasks/{task_id}/attachments/{attachment_id}/download": {
            "get": secured_operation_with_params(
                "Attachments",
                "下载附件",
                "下载任务附件",
                vec![
                    path_uuid_param("task_id", "任务 ID"),
                    path_uuid_param("attachment_id", "附件 ID")
                ],
                Value::Null,
                binary_response()
            )
        },
        "/api/tasks/{task_id}/attachments/{attachment_id}": {
            "delete": secured_operation_with_params(
                "Attachments",
                "删除附件",
                "上传人、管理人员或系统管理员删除任务附件",
                vec![
                    path_uuid_param("task_id", "任务 ID"),
                    path_uuid_param("attachment_id", "附件 ID")
                ],
                Value::Null,
                response_ref("TaskAttachment")
            )
        },
        "/api/notification-records": {
            "get": secured_operation("Notifications", "通知记录", "系统管理员查看钉钉个人通知发送记录", Value::Null, array_response_ref("NotificationRecord"))
        },
        "/api/notification-rules": {
            "get": secured_operation("Notifications", "通知规则", "系统管理员查看任务通知规则开关", Value::Null, array_response_ref("NotificationRule"))
        },
        "/api/notification-rules/{rule_type}": {
            "patch": secured_operation_with_params(
                "Notifications",
                "更新通知规则",
                "系统管理员启用或停用任务通知规则",
                vec![path_param("rule_type", "string", "通知规则类型")],
                json_ref_body("UpdateNotificationRuleRequest"),
                response_ref("NotificationRule")
            )
        }
    })
}

fn components() -> Value {
    json!({
        "securitySchemes": {
            "cookieAuth": {
                "type": "apiKey",
                "in": "cookie",
                "name": "costrategy_session"
            }
        },
        "schemas": {
            "ApiErrorResponse": object_schema(vec![
                required_prop("error", object_schema(vec![
                    required_prop("code", string_schema()),
                    required_prop("message", string_schema()),
                    optional_prop("details", json!({ "type": "object", "additionalProperties": true }))
                ]))
            ]),
            "HealthResponse": object_schema(vec![required_prop("status", string_schema())]),
            "DingtalkLoginRequest": object_schema(vec![required_prop("code", string_schema())]),
            "AdminTokenLoginRequest": object_schema(vec![required_prop("token", string_schema())]),
            "CurrentUser": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("name", string_schema()),
                required_prop("role", enum_schema(vec!["employee", "manager", "admin"])),
                required_prop("departments", json!({ "type": "array", "items": string_schema() })),
                required_prop("permissions", json!({ "type": "array", "items": string_schema() }))
            ]),
            "DingtalkSyncResult": object_schema(vec![
                required_prop("synced_departments", integer_schema()),
                required_prop("synced_users", integer_schema()),
                required_prop("disabled_users", integer_schema())
            ]),
            "DingtalkSyncLog": object_schema(vec![
                required_prop("status", enum_schema(vec!["running", "success", "failed"])),
                required_prop("created_users", integer_schema()),
                required_prop("updated_users", integer_schema()),
                required_prop("disabled_users", integer_schema()),
                optional_prop("failure_reason", string_schema())
            ]),
            "Project": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("name", string_schema()),
                optional_prop("description", string_schema()),
                required_prop("status", enum_schema(vec!["active", "archived"])),
                optional_prop("owner_id", uuid_schema())
            ]),
            "ProjectPayload": object_schema(vec![
                required_prop("name", string_schema()),
                optional_prop("description", string_schema()),
                optional_prop("owner_id", uuid_schema())
            ]),
            "User": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("dingtalk_user_id", string_schema()),
                optional_prop("union_id", string_schema()),
                required_prop("name", string_schema()),
                optional_prop("avatar_url", string_schema()),
                optional_prop("mobile", string_schema()),
                optional_prop("departments", json!({ "type": "array", "items": string_schema() })),
                required_prop("role", enum_schema(vec!["employee", "manager", "admin"])),
                required_prop("status", enum_schema(vec!["active", "disabled"])),
                optional_prop("last_synced_at", datetime_schema())
            ]),
            "UpdateUserRoleRequest": object_schema(vec![
                required_prop("role", enum_schema(vec!["employee", "manager", "admin"]))
            ]),
            "UpdateUserStatusRequest": object_schema(vec![
                required_prop("status", enum_schema(vec!["active", "disabled"]))
            ]),
            "SettingsResponse": object_schema(vec![
                required_prop("settings", array_ref("SettingItem")),
                required_prop("connection_status", object_schema(vec![
                    required_prop("rustfs", string_schema()),
                    required_prop("dingtalk", string_schema())
                ]))
            ]),
            "SettingItem": object_schema(vec![
                required_prop("key", string_schema()),
                required_prop("label", string_schema()),
                required_prop("group", enum_schema(vec!["dingtalk", "rustfs"])),
                required_prop("sensitive", json!({ "type": "boolean" })),
                required_prop("configured", json!({ "type": "boolean" })),
                required_prop("source", enum_schema(vec!["database", "env", "empty"])),
                optional_prop("value_masked", string_schema()),
                optional_prop("updated_by", uuid_schema()),
                optional_prop("updated_at", datetime_schema())
            ]),
            "SettingsUpdateRequest": object_schema(vec![
                required_prop("settings", json!({
                    "type": "array",
                    "items": object_schema(vec![
                        required_prop("key", string_schema()),
                        required_prop("value", string_schema())
                    ])
                }))
            ]),
            "Task": task_schema(),
            "TaskPayload": object_schema(vec![
                required_prop("project_id", uuid_schema()),
                required_prop("title", string_schema()),
                required_prop("assignee_id", uuid_schema()),
                optional_prop(
                    "assignee_ids",
                    json!({ "type": "array", "items": uuid_schema() }),
                ),
                required_prop("status", enum_schema(vec!["todo", "in_progress", "done"])),
                required_prop("priority", enum_schema(vec!["low", "medium", "high"])),
                required_prop("start_date", date_schema()),
                required_prop("due_date", date_schema()),
                required_prop("description_json", json!({ "type": "object", "additionalProperties": true }))
            ]),
            "UpdateTaskStatusRequest": object_schema(vec![
                required_prop("status", enum_schema(vec!["todo", "in_progress", "done"]))
            ]),
            "TaskDetail": object_schema(vec![
                required_prop("task", schema_ref("Task")),
                required_prop("comments", array_ref("TaskComment")),
                required_prop("attachments", array_ref("TaskAttachment")),
                required_prop("activity_logs", array_ref("TaskActivityLog"))
            ]),
            "CreateTaskCommentRequest": object_schema(vec![required_prop("content", string_schema())]),
            "TaskComment": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("task_id", uuid_schema()),
                required_prop("author_id", uuid_schema()),
                optional_prop("author_name", string_schema()),
                required_prop("content", string_schema()),
                required_prop("created_at", datetime_schema())
            ]),
            "TaskAttachment": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("task_id", uuid_schema()),
                required_prop("file_name", string_schema()),
                required_prop("file_size", integer_schema()),
                optional_prop("mime_type", string_schema()),
                required_prop("uploader_id", uuid_schema()),
                optional_prop("uploader_name", string_schema()),
                required_prop("created_at", datetime_schema())
            ]),
            "RichTextImageUploadResponse": object_schema(vec![
                required_prop("url", string_schema())
            ]),
            "TaskActivityLog": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("task_id", uuid_schema()),
                optional_prop("actor_id", uuid_schema()),
                optional_prop("actor_name", string_schema()),
                required_prop("action", string_schema()),
                required_prop("created_at", datetime_schema())
            ]),
            "NotificationRecord": object_schema(vec![
                required_prop("id", uuid_schema()),
                required_prop("notification_type", string_schema()),
                required_prop("receiver_id", uuid_schema()),
                optional_prop("task_id", uuid_schema()),
                required_prop("content_summary", string_schema()),
                required_prop("status", enum_schema(vec!["success", "failed"])),
                optional_prop("failure_reason", string_schema()),
                required_prop("sent_at", datetime_schema())
            ]),
            "NotificationRule": object_schema(vec![
                required_prop("rule_type", enum_schema(vec!["task_assigned", "assignee_changed", "due_tomorrow", "task_overdue"])),
                required_prop("enabled", json!({ "type": "boolean" })),
                optional_prop("updated_by", uuid_schema()),
                optional_prop("updated_at", datetime_schema())
            ]),
            "UpdateNotificationRuleRequest": object_schema(vec![
                required_prop("enabled", json!({ "type": "boolean" }))
            ])
        },
        "responses": {
            "ApiError": {
                "description": "统一错误响应",
                "content": {
                    "application/json": {
                        "schema": schema_ref("ApiErrorResponse")
                    }
                }
            }
        }
    })
}

fn operation(
    tag: &str,
    summary: &str,
    description: &str,
    request_body: Value,
    ok_response: Value,
) -> Value {
    operation_with_params(
        tag,
        summary,
        description,
        Vec::new(),
        request_body,
        ok_response,
        false,
    )
}

fn secured_operation(
    tag: &str,
    summary: &str,
    description: &str,
    request_body: Value,
    ok_response: Value,
) -> Value {
    operation_with_params(
        tag,
        summary,
        description,
        Vec::new(),
        request_body,
        ok_response,
        true,
    )
}

fn secured_operation_with_params(
    tag: &str,
    summary: &str,
    description: &str,
    parameters: Vec<Value>,
    request_body: Value,
    ok_response: Value,
) -> Value {
    operation_with_params(
        tag,
        summary,
        description,
        parameters,
        request_body,
        ok_response,
        true,
    )
}

fn operation_with_params(
    tag: &str,
    summary: &str,
    description: &str,
    parameters: Vec<Value>,
    request_body: Value,
    ok_response: Value,
    secured: bool,
) -> Value {
    let mut operation = json!({
        "tags": [tag],
        "summary": summary,
        "description": description,
        "responses": {
            "200": ok_response,
            "400": { "$ref": "#/components/responses/ApiError" },
            "401": { "$ref": "#/components/responses/ApiError" },
            "403": { "$ref": "#/components/responses/ApiError" },
            "404": { "$ref": "#/components/responses/ApiError" },
            "500": { "$ref": "#/components/responses/ApiError" }
        }
    });

    if !parameters.is_empty() {
        operation["parameters"] = Value::Array(parameters);
    }

    if !request_body.is_null() {
        operation["requestBody"] = request_body;
    }

    if secured {
        operation["security"] = json!([{ "cookieAuth": [] }]);
    }

    operation
}

fn path_uuid_param(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "path",
        "required": true,
        "description": description,
        "schema": uuid_schema()
    })
}

fn path_param(name: &str, type_name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "path",
        "required": true,
        "description": description,
        "schema": { "type": type_name }
    })
}

fn query_param(name: &str, type_name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "query",
        "required": false,
        "description": description,
        "schema": { "type": type_name }
    })
}

fn json_ref_body(schema_name: &str) -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": schema_ref(schema_name)
            }
        }
    })
}

fn multipart_file_body() -> Value {
    json!({
        "required": true,
        "content": {
            "multipart/form-data": {
                "schema": {
                    "type": "object",
                    "required": ["file"],
                    "properties": {
                        "file": {
                            "type": "string",
                            "format": "binary"
                        }
                    }
                }
            }
        }
    })
}

fn response_ref(schema_name: &str) -> Value {
    json!({
        "description": "成功",
        "content": {
            "application/json": {
                "schema": schema_ref(schema_name)
            }
        }
    })
}

fn array_response_ref(schema_name: &str) -> Value {
    json!({
        "description": "成功",
        "content": {
            "application/json": {
                "schema": array_ref(schema_name)
            }
        }
    })
}

fn binary_response() -> Value {
    json!({
        "description": "附件二进制内容",
        "content": {
            "application/octet-stream": {
                "schema": {
                    "type": "string",
                    "format": "binary"
                }
            }
        }
    })
}

fn empty_response() -> Value {
    json!({ "description": "成功" })
}

fn task_schema() -> Value {
    object_schema(vec![
        required_prop("id", uuid_schema()),
        required_prop("project_id", uuid_schema()),
        optional_prop("project_name", string_schema()),
        required_prop("title", string_schema()),
        required_prop("assignee_id", uuid_schema()),
        optional_prop("assignee_name", string_schema()),
        required_prop(
            "assignees",
            json!({
                "type": "array",
                "items": object_schema(vec![
                    required_prop("id", uuid_schema()),
                    optional_prop("name", string_schema())
                ])
            }),
        ),
        required_prop("status", enum_schema(vec!["todo", "in_progress", "done"])),
        required_prop("priority", enum_schema(vec!["low", "medium", "high"])),
        required_prop("start_date", date_schema()),
        required_prop("due_date", date_schema()),
        required_prop(
            "description_json",
            json!({ "type": "object", "additionalProperties": true }),
        ),
        required_prop("creator_id", uuid_schema()),
        optional_prop("creator_name", string_schema()),
        required_prop("archived", json!({ "type": "boolean" })),
        required_prop("is_overdue", json!({ "type": "boolean" })),
        required_prop("display_status", string_schema()),
    ])
}

fn object_schema(properties: Vec<(&'static str, Value, bool)>) -> Value {
    let required = properties
        .iter()
        .filter_map(|(name, _, is_required)| is_required.then_some(Value::String((*name).into())))
        .collect::<Vec<_>>();
    let mut schema = json!({
        "type": "object",
        "properties": properties
            .into_iter()
            .map(|(name, value, _)| (name.to_string(), value))
            .collect::<serde_json::Map<_, _>>()
    });

    if !required.is_empty() {
        schema["required"] = Value::Array(required);
    }

    schema
}

fn required_prop(name: &'static str, schema: Value) -> (&'static str, Value, bool) {
    (name, schema, true)
}

fn optional_prop(name: &'static str, schema: Value) -> (&'static str, Value, bool) {
    (name, schema, false)
}

fn schema_ref(schema_name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{schema_name}") })
}

fn array_ref(schema_name: &str) -> Value {
    json!({
        "type": "array",
        "items": schema_ref(schema_name)
    })
}

fn string_schema() -> Value {
    json!({ "type": "string" })
}

fn uuid_schema() -> Value {
    json!({ "type": "string", "format": "uuid" })
}

fn date_schema() -> Value {
    json!({ "type": "string", "format": "date" })
}

fn datetime_schema() -> Value {
    json!({ "type": "string", "format": "date-time" })
}

fn integer_schema() -> Value {
    json!({ "type": "integer", "format": "int64" })
}

fn enum_schema(values: Vec<&str>) -> Value {
    json!({
        "type": "string",
        "enum": values
    })
}

const SWAGGER_UI_HTML: &str = r##"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>项目管理系统 API 文档</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
    <style>
      body { margin: 0; background: #f7f8fb; }
      .swagger-ui .topbar { display: none; }
    </style>
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({
        url: "/api-docs/openapi.json",
        dom_id: "#swagger-ui",
        deepLinking: true,
        persistAuthorization: true
      });
    </script>
  </body>
</html>"##;
