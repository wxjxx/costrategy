import axios from "axios";
import type {
  CurrentUser,
  DingtalkSyncLog,
  NotificationRecord,
  NotificationRule,
  Project,
  ProjectPayload,
  SettingsResponse,
  Task,
  TaskDetail,
  TaskFilters,
  TaskPayload,
  TaskStatus,
  User,
  UserRole,
  UserStatus,
} from "@/types";

const baseURL = import.meta.env.VITE_API_BASE_URL || "/api";
type UnauthorizedRedirectHandler = (path: string) => void;
let unauthorizedRedirectHandler: UnauthorizedRedirectHandler | undefined;

export const http = axios.create({
  baseURL,
  withCredentials: true,
  timeout: 10_000,
});

export function setUnauthorizedRedirectHandler(
  handler: UnauthorizedRedirectHandler | undefined,
): void {
  unauthorizedRedirectHandler = handler;
}

export function redirectUnauthorizedError(error: unknown): Promise<never> {
  if (axios.isAxiosError(error) && error.response?.status === 401) {
    unauthorizedRedirectHandler?.("/401");
  } else if (
    typeof error === "object" &&
    error !== null &&
    "response" in error &&
    (error as { response?: { status?: number } }).response?.status === 401
  ) {
    unauthorizedRedirectHandler?.("/401");
  }
  return Promise.reject(error);
}

http.interceptors.response.use((response) => response, redirectUnauthorizedError);

export function buildTaskQueryParams(filters: TaskFilters): URLSearchParams {
  const params = new URLSearchParams();
  Object.entries(filters).forEach(([key, value]) => {
    if (value !== undefined && value !== "") {
      params.set(key, String(value));
    }
  });
  return params;
}

export const api = {
  me: async (): Promise<CurrentUser> => (await http.get("/me")).data,
  tasks: async (filters: TaskFilters = {}): Promise<Task[]> =>
    (await http.get("/tasks", { params: buildTaskQueryParams(filters) })).data,
  taskDetail: async (taskId: string): Promise<TaskDetail> =>
    (await http.get(`/tasks/${taskId}`)).data,
  updateTaskStatus: async (taskId: string, status: TaskStatus): Promise<Task> =>
    (await http.patch(`/tasks/${taskId}/status`, { status })).data,
  createTask: async (payload: TaskPayload): Promise<Task> =>
    (await http.post("/tasks", payload)).data,
  updateTask: async (taskId: string, payload: TaskPayload): Promise<Task> =>
    (await http.put(`/tasks/${taskId}`, payload)).data,
  projects: async (): Promise<Project[]> => (await http.get("/projects")).data,
  createProject: async (payload: ProjectPayload): Promise<Project> =>
    (await http.post("/projects", payload)).data,
  updateProject: async (
    projectId: string,
    payload: ProjectPayload,
  ): Promise<Project> => (await http.put(`/projects/${projectId}`, payload)).data,
  archiveProject: async (projectId: string): Promise<Project> =>
    (await http.post(`/projects/${projectId}/archive`)).data,
  users: async (): Promise<User[]> => (await http.get("/users")).data,
  updateUserRole: (userId: string, role: UserRole): Promise<User> =>
    http.patch(`/users/${userId}/role`, { role }).then((response) => response.data),
  updateUserStatus: (userId: string, status: UserStatus): Promise<User> =>
    http.patch(`/users/${userId}/status`, { status }).then((response) => response.data),
  settings: async (): Promise<SettingsResponse> => (await http.get("/settings")).data,
  syncLogs: async (): Promise<DingtalkSyncLog[]> =>
    (await http.get("/dingtalk/sync-logs")).data,
  syncDingtalk: async (): Promise<{
    synced_departments: number;
    synced_users: number;
    disabled_users: number;
  }> => (await http.post("/dingtalk/sync")).data,
  notificationRules: async (): Promise<NotificationRule[]> =>
    (await http.get("/notification-rules")).data,
  updateNotificationRule: async (
    ruleType: NotificationRule["rule_type"],
    enabled: boolean,
  ): Promise<NotificationRule> =>
    (await http.patch(`/notification-rules/${ruleType}`, { enabled })).data,
  notificationRecords: async (): Promise<NotificationRecord[]> =>
    (await http.get("/notification-records")).data,
};
