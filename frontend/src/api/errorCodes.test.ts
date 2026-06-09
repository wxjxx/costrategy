import {
  API_ERROR_MESSAGES,
  API_ERROR_CODES,
  getApiErrorMessage,
  type ApiErrorResponse,
} from './errorCodes';

test('contains the same stable error codes defined by backend plan', () => {
  expect(API_ERROR_CODES).toEqual([
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
  ]);

  expect(Object.keys(API_ERROR_MESSAGES)).toEqual(API_ERROR_CODES);
});

test('maps backend error code to frontend chinese message', () => {
  const response: ApiErrorResponse = {
    error: {
      code: 'TASK_NOT_ASSIGNEE',
      message: 'backend fallback message',
    },
  };

  expect(getApiErrorMessage(response)).toBe('只能更新自己负责的任务');
});

test('falls back to backend message for unknown error payloads', () => {
  expect(
    getApiErrorMessage({
      error: {
        code: 'NEW_BACKEND_CODE',
        message: '服务端新错误',
      },
    }),
  ).toBe('服务端新错误');
});
