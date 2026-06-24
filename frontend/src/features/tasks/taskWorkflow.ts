import type {
  CurrentUser,
  DisplayStatus,
  Task,
  TaskFilters,
  TaskStatus,
} from "@/types";

export const displayColumns: Array<{
  key: DisplayStatus;
  title: string;
  dotClass: string;
}> = [
  { key: "todo", title: "待开始", dotClass: "is-gray" },
  { key: "in_progress", title: "进行中", dotClass: "is-blue" },
  { key: "done", title: "已完成", dotClass: "is-green" },
  { key: "blocked", title: "阻塞", dotClass: "is-orange" },
];

type StatusColorKey = TaskStatus | "overdue";

const taskStatusColors: Record<StatusColorKey, string> = {
  todo: "#8b95a5",
  in_progress: "#2b7bff",
  blocked: "#f97316",
  done: "#35b86b",
  overdue: "#ff4d4f",
};

export function getDisplayStatus(task: Task): DisplayStatus {
  return task.status;
}

export function statusColor(status: StatusColorKey): string {
  return taskStatusColors[status];
}

export function taskDisplayStatusColor(task: Task): string {
  return statusColor(task.is_overdue ? "overdue" : task.status);
}

export function groupTasksByDisplayStatus(
  tasks: Task[],
): Record<DisplayStatus, Task[]> {
  return tasks.reduce<Record<DisplayStatus, Task[]>>(
    (groups, task) => {
      groups[getDisplayStatus(task)].push(task);
      return groups;
    },
    { todo: [], in_progress: [], blocked: [], done: [] },
  );
}

export function moveTaskForDisplay(
  tasks: Task[],
  taskId: string,
  status: TaskStatus,
): Task[] {
  return tasks.map((task) =>
    task.id === taskId ? { ...task, status } : task,
  );
}

export function canManageTasks(user: CurrentUser): boolean {
  return user.role === "manager" || user.role === "admin";
}

export function canMoveTaskToStatus(
  task: Task,
  _targetStatus: DisplayStatus,
  user: CurrentUser,
): _targetStatus is TaskStatus {
  return canManageTasks(user) || taskAssigneeIds(task).includes(user.id);
}

export function filterTasks(tasks: Task[], filters: TaskFilters): Task[] {
  const keyword = filters.keyword?.trim().toLowerCase();
  const projectIds = selectedValues(filters.project_ids, filters.project_id);
  const assigneeIds = selectedValues(filters.assignee_ids, filters.assignee_id);
  const statuses = selectedValues(filters.statuses, filters.status);
  const priorities = selectedValues(filters.priorities, filters.priority);
  return tasks.filter((task) => {
    if (projectIds.length && !projectIds.includes(task.project_id)) return false;
    if (
      assigneeIds.length &&
      !taskAssigneeIds(task).some((assigneeId) => assigneeIds.includes(assigneeId))
    ) return false;
    if (statuses.length && !statuses.includes(task.status)) return false;
    if (priorities.length && !priorities.includes(task.priority)) return false;
    if (keyword && !task.title.toLowerCase().includes(keyword)) return false;
    if (filters.date_from && task.due_date < filters.date_from) return false;
    if (filters.date_to && task.start_date > filters.date_to) return false;
    return true;
  });
}

function selectedValues<T>(many?: T[], single?: T): T[] {
  if (many?.length) return many;
  return single ? [single] : [];
}

export function taskAssigneeIds(task: Task): string[] {
  const ids = task.assignees?.map((assignee) => assignee.id).filter(Boolean) ?? [];
  return ids.length > 0 ? ids : [task.assignee_id];
}

export function taskAssigneeNames(task: Task): string {
  const names = task.assignees
    ?.map((assignee) => assignee.name)
    .filter((name): name is string => Boolean(name?.trim()));
  if (names?.length) return names.join("、");
  return task.assignee_name || "-";
}

export function primaryTaskAssigneeName(task: Task): string | undefined {
  return task.assignees?.[0]?.name ?? task.assignee_name;
}

export function statusLabel(status: DisplayStatus): string {
  const labels: Record<DisplayStatus, string> = {
    todo: "待开始",
    in_progress: "进行中",
    blocked: "阻塞",
    done: "已完成",
  };
  return labels[status];
}

export function priorityLabel(priority: Task["priority"]): string {
  return { low: "低", medium: "中", high: "高" }[priority];
}
