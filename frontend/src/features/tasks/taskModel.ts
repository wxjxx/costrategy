export type TaskStatus = 'todo' | 'in_progress' | 'done';
export type TaskPriority = 'low' | 'medium' | 'high';

export interface TaskItem {
  id: string;
  project_id: string;
  project_name?: string;
  title: string;
  assignee_id: string;
  assignee_name?: string;
  status: TaskStatus;
  priority: TaskPriority;
  start_date: string;
  due_date: string;
  description_json: unknown;
  creator_id: string;
  archived: boolean;
  is_overdue: boolean;
  display_status: string;
}

export interface TaskComment {
  id: string;
  task_id: string;
  author_id: string;
  author_name?: string;
  content: string;
  created_at: string;
}

export interface TaskAttachmentSummary {
  id: string;
  task_id: string;
  file_name: string;
  file_size: number;
  mime_type?: string;
  uploader_id: string;
  uploader_name?: string;
  created_at: string;
}

export interface TaskActivityLog {
  id: string;
  task_id: string;
  actor_id?: string;
  actor_name?: string;
  action: string;
  created_at: string;
}

export interface TaskDetail {
  task: TaskItem;
  comments: TaskComment[];
  attachments: TaskAttachmentSummary[];
  activity_logs: TaskActivityLog[];
}

export interface TaskFilters {
  keyword?: string;
  project_id?: string;
  assignee_id?: string;
  status?: TaskStatus | '';
  priority?: TaskPriority | '';
  date_from?: string;
  date_to?: string;
}

export interface UpdateTaskPayload {
  project_id: string;
  title: string;
  assignee_id: string;
  status: TaskStatus;
  priority: TaskPriority;
  start_date: string;
  due_date: string;
  description_json: unknown;
}

export type CreateTaskPayload = UpdateTaskPayload;

export const TASK_STATUS_COLUMNS: Array<{ status: TaskStatus; label: string }> = [
  { status: 'todo', label: '待处理' },
  { status: 'in_progress', label: '进行中' },
  { status: 'done', label: '已完成' },
];

export const TASK_PRIORITY_OPTIONS: Array<{ priority: TaskPriority; label: string }> = [
  { priority: 'low', label: '低' },
  { priority: 'medium', label: '中' },
  { priority: 'high', label: '高' },
];

export type TaskStatusGroups = Record<TaskStatus, TaskItem[]>;

export function groupTasksByStatus(tasks: TaskItem[]): TaskStatusGroups {
  return tasks.reduce<TaskStatusGroups>(
    (groups, task) => {
      groups[task.status].push(task);
      return groups;
    },
    {
      todo: [],
      in_progress: [],
      done: [],
    },
  );
}

export function taskStatusLabel(status: TaskStatus): string {
  return TASK_STATUS_COLUMNS.find((column) => column.status === status)?.label ?? status;
}

export function taskPriorityLabel(priority: TaskPriority): string {
  return TASK_PRIORITY_OPTIONS.find((option) => option.priority === priority)?.label ?? priority;
}

export function taskProjectLabel(task: TaskItem): string {
  return task.project_name || task.project_id;
}

export function taskAssigneeLabel(task: TaskItem): string {
  return task.assignee_name || task.assignee_id;
}

export function taskActivityLabel(action: string): string {
  const labels: Record<string, string> = {
    task_created: '创建任务',
    assignee_changed: '调整负责人',
    status_changed: '更新状态',
    schedule_changed: '调整排期',
    priority_changed: '调整优先级',
    task_archived: '归档任务',
    attachment_uploaded: '上传附件',
    attachment_deleted: '删除附件',
    comment_created: '新增评论',
  };

  return labels[action] ?? action;
}

export function taskPersonLabel(name: string | undefined, id: string | undefined): string {
  return name || id || '系统';
}

export function extractTaskDescriptionText(description: unknown): string {
  const lines = collectDescriptionLines(description).map((line) => line.trim());
  return lines.filter((line) => line.length > 0).join('\n');
}

export function countActiveTaskFilters(filters: TaskFilters): number {
  return Object.values(filters).filter((value) => {
    if (typeof value !== 'string') {
      return Boolean(value);
    }

    return value.trim().length > 0;
  }).length;
}

export function replaceTaskStatus(
  tasks: TaskItem[],
  taskId: string,
  status: TaskStatus,
): TaskItem[] {
  return tasks.map((task) =>
    task.id === taskId
      ? {
          ...task,
          status,
          display_status: status,
          is_overdue: status === 'done' ? false : task.is_overdue,
        }
      : task,
  );
}

function collectDescriptionLines(node: unknown): string[] {
  if (!node || typeof node !== 'object') {
    return [];
  }

  const record = node as { type?: unknown; text?: unknown; content?: unknown };
  if (typeof record.text === 'string') {
    return [record.text];
  }

  if (!Array.isArray(record.content)) {
    return [];
  }

  if (record.type === 'paragraph' || record.type === 'heading') {
    return [record.content.flatMap(collectDescriptionLines).join('')];
  }

  return record.content.flatMap(collectDescriptionLines);
}
