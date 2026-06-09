export const API_ERROR_CODES = [
  'AUTH_NOT_LOGIN',
  'AUTH_FORBIDDEN',
  'AUTH_DINGTALK_LOGIN_FAILED',
  'AUTH_USER_NOT_SYNCED',
  'AUTH_USER_DISABLED',
  'VALIDATION_FAILED',
  'RESOURCE_NOT_FOUND',
  'CONFIG_MISSING',
  'DATABASE_ERROR',
  'INTERNAL_ERROR',
  'PROJECT_ARCHIVED',
  'TASK_INVALID_STATUS_TRANSITION',
  'TASK_NOT_ASSIGNEE',
  'TASK_ASSIGNEE_INACTIVE',
  'TASK_DATE_RANGE_INVALID',
  'ATTACHMENT_UPLOAD_FAILED',
  'ATTACHMENT_DELETE_FORBIDDEN',
  'STORAGE_CONFIG_INVALID',
  'STORAGE_DOWNLOAD_FAILED',
  'DINGTALK_CONFIG_MISSING',
  'DINGTALK_SYNC_FAILED',
  'DINGTALK_NOTIFY_FAILED',
] as const;

export type ApiErrorCode = (typeof API_ERROR_CODES)[number];

export interface ApiErrorResponse {
  error: {
    code: ApiErrorCode | string;
    message: string;
    details?: Record<string, unknown>;
  };
}

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

export function getApiErrorMessage(response: ApiErrorResponse): string {
  if (isApiErrorCode(response.error.code)) {
    return API_ERROR_MESSAGES[response.error.code];
  }

  return response.error.message || API_ERROR_MESSAGES.INTERNAL_ERROR;
}

export function isApiErrorCode(code: string): code is ApiErrorCode {
  return (API_ERROR_CODES as readonly string[]).includes(code);
}
