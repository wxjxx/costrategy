use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorBody {
    pub code: ApiErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    status: StatusCode,
    code: ApiErrorCode,
    details: Option<BTreeMap<String, Value>>,
}

impl AppError {
    pub fn new(status: StatusCode, code: ApiErrorCode) -> Self {
        Self {
            status,
            code,
            details: None,
        }
    }

    pub fn forbidden(code: ApiErrorCode) -> Self {
        Self::new(StatusCode::FORBIDDEN, code)
    }

    pub fn bad_request(code: ApiErrorCode) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code)
    }

    pub fn internal(code: ApiErrorCode) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code)
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.details
            .get_or_insert_with(BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    pub fn body(&self) -> ApiErrorResponse {
        ApiErrorResponse {
            error: ApiErrorBody {
                code: self.code,
                message: self.code.default_message().to_string(),
                details: self.details.clone(),
            },
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    pub fn code(&self) -> ApiErrorCode {
        self.code
    }
}

impl ApiErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthNotLogin => "AUTH_NOT_LOGIN",
            Self::AuthForbidden => "AUTH_FORBIDDEN",
            Self::AuthDingtalkLoginFailed => "AUTH_DINGTALK_LOGIN_FAILED",
            Self::AuthUserNotSynced => "AUTH_USER_NOT_SYNCED",
            Self::AuthUserDisabled => "AUTH_USER_DISABLED",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::ResourceNotFound => "RESOURCE_NOT_FOUND",
            Self::ConfigMissing => "CONFIG_MISSING",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::InternalError => "INTERNAL_ERROR",
            Self::ProjectArchived => "PROJECT_ARCHIVED",
            Self::TaskInvalidStatusTransition => "TASK_INVALID_STATUS_TRANSITION",
            Self::TaskNotAssignee => "TASK_NOT_ASSIGNEE",
            Self::TaskAssigneeInactive => "TASK_ASSIGNEE_INACTIVE",
            Self::TaskDateRangeInvalid => "TASK_DATE_RANGE_INVALID",
            Self::AttachmentUploadFailed => "ATTACHMENT_UPLOAD_FAILED",
            Self::AttachmentDeleteForbidden => "ATTACHMENT_DELETE_FORBIDDEN",
            Self::StorageConfigInvalid => "STORAGE_CONFIG_INVALID",
            Self::StorageDownloadFailed => "STORAGE_DOWNLOAD_FAILED",
            Self::DingtalkConfigMissing => "DINGTALK_CONFIG_MISSING",
            Self::DingtalkSyncFailed => "DINGTALK_SYNC_FAILED",
            Self::DingtalkNotifyFailed => "DINGTALK_NOTIFY_FAILED",
        }
    }

    pub fn default_message(self) -> &'static str {
        match self {
            Self::AuthNotLogin => "请先登录",
            Self::AuthForbidden => "当前账号没有操作权限",
            Self::AuthDingtalkLoginFailed => "钉钉免登失败，请重新从钉钉工作台进入",
            Self::AuthUserNotSynced => "当前钉钉用户尚未同步到系统",
            Self::AuthUserDisabled => "当前账号已停用",
            Self::ValidationFailed => "提交内容不符合要求",
            Self::ResourceNotFound => "数据不存在或已被删除",
            Self::ConfigMissing => "系统配置缺失，请联系管理员",
            Self::DatabaseError => "数据库操作失败",
            Self::InternalError => "系统异常，请稍后重试",
            Self::ProjectArchived => "项目已归档，不能继续操作",
            Self::TaskInvalidStatusTransition => "任务状态流转不允许",
            Self::TaskNotAssignee => "只能更新自己负责的任务",
            Self::TaskAssigneeInactive => "负责人账号不可用",
            Self::TaskDateRangeInvalid => "开始日期不能晚于截止日期",
            Self::AttachmentUploadFailed => "附件上传失败",
            Self::AttachmentDeleteForbidden => "没有权限删除该附件",
            Self::StorageConfigInvalid => "文件存储配置不可用",
            Self::StorageDownloadFailed => "附件下载失败",
            Self::DingtalkConfigMissing => "钉钉应用配置缺失",
            Self::DingtalkSyncFailed => "钉钉通讯录同步失败",
            Self::DingtalkNotifyFailed => "钉钉通知发送失败",
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.status.as_u16(),
            self.code.as_str(),
            self.code.default_message()
        )
    }
}

impl std::error::Error for AppError {}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        log::error!(
            "api request failed: status={}, code={}, details={:?}",
            self.status.as_u16(),
            self.code.as_str(),
            self.details
        );
        HttpResponse::build(self.status).json(self.body())
    }
}
