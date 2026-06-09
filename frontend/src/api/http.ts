import axios from 'axios';
import { getApiErrorMessage, type ApiErrorResponse } from './errorCodes';

export const http = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL ?? '/api',
  withCredentials: true,
});

export function getHttpErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error) && isApiErrorResponse(error.response?.data)) {
    return getApiErrorMessage(error.response.data);
  }

  return '系统异常，请稍后重试';
}

function isApiErrorResponse(payload: unknown): payload is ApiErrorResponse {
  if (!payload || typeof payload !== 'object') {
    return false;
  }

  const maybeError = (payload as { error?: unknown }).error;
  if (!maybeError || typeof maybeError !== 'object') {
    return false;
  }

  return typeof (maybeError as { code?: unknown }).code === 'string';
}
