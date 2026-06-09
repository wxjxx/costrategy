import { http } from '../../api/http';
import type {
  CreateTaskPayload,
  TaskAttachmentSummary,
  TaskComment,
  TaskDetail,
  TaskFilters,
  TaskItem,
  TaskStatus,
  UpdateTaskPayload,
} from './taskModel';

export function buildTaskQueryParams(filters: TaskFilters): URLSearchParams {
  const params = new URLSearchParams();

  Object.entries(filters).forEach(([key, value]) => {
    if (typeof value !== 'string') {
      return;
    }

    const trimmed = value.trim();
    if (trimmed.length > 0) {
      params.set(key, trimmed);
    }
  });

  return params;
}

export async function fetchTasks(filters: TaskFilters): Promise<TaskItem[]> {
  const response = await http.get<TaskItem[]>('/tasks', {
    params: buildTaskQueryParams(filters),
  });
  return response.data;
}

export async function createTask(payload: CreateTaskPayload): Promise<TaskItem> {
  const response = await http.post<TaskItem>('/tasks', payload);
  return response.data;
}

export async function updateTaskStatus(taskId: string, status: TaskStatus): Promise<TaskItem> {
  const response = await http.patch<TaskItem>(`/tasks/${taskId}/status`, { status });
  return response.data;
}

export async function updateTask(
  taskId: string,
  payload: UpdateTaskPayload,
): Promise<TaskItem> {
  const response = await http.put<TaskItem>(`/tasks/${taskId}`, payload);
  return response.data;
}

export async function fetchTaskDetail(taskId: string): Promise<TaskDetail> {
  const response = await http.get<TaskDetail>(`/tasks/${taskId}`);
  return response.data;
}

export async function createTaskComment(taskId: string, content: string): Promise<TaskComment> {
  const response = await http.post<TaskComment>(`/tasks/${taskId}/comments`, { content });
  return response.data;
}

export async function uploadTaskAttachment(
  taskId: string,
  file: File,
): Promise<TaskAttachmentSummary> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await http.post<TaskAttachmentSummary>(`/tasks/${taskId}/attachments`, formData);
  return response.data;
}

export async function deleteTaskAttachment(
  taskId: string,
  attachmentId: string,
): Promise<TaskAttachmentSummary> {
  const response = await http.delete<TaskAttachmentSummary>(
    `/tasks/${taskId}/attachments/${attachmentId}`,
  );
  return response.data;
}

export function getAttachmentDownloadUrl(taskId: string, attachmentId: string): string {
  const baseUrl = String(http.defaults.baseURL ?? '/api').replace(/\/$/, '');
  return `${baseUrl}/tasks/${encodeURIComponent(taskId)}/attachments/${encodeURIComponent(
    attachmentId,
  )}/download`;
}
