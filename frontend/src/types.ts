export type UserRole = "employee" | "manager" | "admin";
export type UserStatus = "active" | "disabled";
export type TaskStatus = "todo" | "in_progress" | "done";
export type TaskPriority = "low" | "medium" | "high";
export type DisplayStatus = TaskStatus | "overdue";
export type ProjectStatus = "active" | "archived" | "completed" | "paused";

export interface CurrentUser {
  id: string;
  name: string;
  role: UserRole;
  departments: string[];
  permissions: string[];
}

export interface User {
  id: string;
  dingtalk_user_id: string;
  union_id?: string;
  name: string;
  avatar_url?: string;
  mobile?: string;
  role: UserRole;
  status: UserStatus;
  departments?: string[];
  last_synced_at?: string;
}

export interface Project {
  id: string;
  code: string;
  name: string;
  description?: string;
  owner_id?: string;
  start_date?: string;
  end_date?: string;
  status: ProjectStatus;
}

export interface CreateProjectPayload {
  code: string;
  name: string;
  description?: string;
  owner_id?: string;
  start_date?: string;
  end_date?: string;
  status: ProjectStatus;
}

export interface UpdateProjectPayload {
  name: string;
  description?: string;
  owner_id?: string;
  start_date?: string;
  end_date?: string;
  status: ProjectStatus;
}

export type ProjectPayload = CreateProjectPayload;

export interface Task {
  id: string;
  project_id: string;
  project_name?: string;
  title: string;
  assignee_id: string;
  assignee_name?: string;
  assignees?: TaskAssignee[];
  status: TaskStatus;
  priority: TaskPriority;
  start_date: string;
  due_date: string;
  description_json: Record<string, unknown>;
  creator_id: string;
  creator_name?: string;
  archived: boolean;
  is_overdue: boolean;
  display_status: string;
}

export interface TaskAssignee {
  id: string;
  name?: string;
}

export interface TaskPayload {
  project_id: string;
  title: string;
  assignee_id: string;
  assignee_ids: string[];
  status: TaskStatus;
  priority: TaskPriority;
  start_date: string;
  due_date: string;
  description_json: Record<string, unknown>;
}

export interface TaskComment {
  id: string;
  task_id: string;
  author_id: string;
  author_name?: string;
  content: string;
  created_at: string;
}

export interface TaskAttachment {
  id: string;
  task_id: string;
  file_name: string;
  file_size: number;
  mime_type?: string;
  uploader_id: string;
  uploader_name?: string;
  created_at: string;
}

export interface RichTextImageUpload {
  url: string;
}

export interface TaskActivityLog {
  id: string;
  task_id: string;
  action: string;
  actor_id?: string;
  actor_name?: string;
  created_at: string;
}

export interface TaskDetail {
  task: Task;
  comments: TaskComment[];
  attachments: TaskAttachment[];
  activity_logs: TaskActivityLog[];
}

export interface TaskFilters {
  keyword?: string;
  project_id?: string;
  assignee_id?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
  date_from?: string;
  date_to?: string;
}

export interface SettingItem {
  key: string;
  label: string;
  group: "dingtalk" | "rustfs";
  sensitive: boolean;
  configured: boolean;
  source: "database" | "env" | "empty";
  value_masked?: string;
  updated_at?: string;
}

export interface SettingsResponse {
  settings: SettingItem[];
  connection_status: {
    dingtalk: string;
    rustfs: string;
  };
}

export interface SettingsUpdatePayload {
  key: string;
  value: string;
}

export interface DingtalkSyncLog {
  status: "running" | "success" | "failed";
  created_users: number;
  updated_users: number;
  disabled_users: number;
  failure_reason?: string;
}

export interface NotificationRule {
  rule_type:
    | "task_assigned"
    | "assignee_changed"
    | "due_tomorrow"
    | "task_overdue";
  enabled: boolean;
  updated_at?: string;
  updated_by?: string;
}

export interface NotificationRecord {
  id: string;
  notification_type: string;
  receiver_id: string;
  task_id?: string;
  jump_url?: string;
  content_summary: string;
  status: "success" | "failed";
  sent_at: string;
  read_at?: string;
  failure_reason?: string;
}
