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
  { key: "overdue", title: "已延期", dotClass: "is-red" },
];

const taskStatusColors: Record<DisplayStatus, string> = {
  todo: "#8b95a5",
  in_progress: "#2b7bff",
  done: "#35b86b",
  overdue: "#ff4d4f",
};

export function getDisplayStatus(task: Task): DisplayStatus {
  if (task.status !== "done" && task.is_overdue) {
    return "overdue";
  }
  return task.status;
}

export function statusColor(status: DisplayStatus | TaskStatus): string {
  return taskStatusColors[status];
}

export function taskDisplayStatusColor(task: Task): string {
  return statusColor(getDisplayStatus(task));
}

export function groupTasksByDisplayStatus(
  tasks: Task[],
): Record<DisplayStatus, Task[]> {
  return tasks.reduce<Record<DisplayStatus, Task[]>>(
    (groups, task) => {
      groups[getDisplayStatus(task)].push(task);
      return groups;
    },
    { todo: [], in_progress: [], done: [], overdue: [] },
  );
}

export function moveTaskForDisplay(
  tasks: Task[],
  taskId: string,
  status: TaskStatus,
): Task[] {
  return tasks.map((task) =>
    task.id === taskId ? { ...task, status, is_overdue: false } : task,
  );
}

export function canManageTasks(user: CurrentUser): boolean {
  return user.role === "manager" || user.role === "admin";
}

export function canMoveTaskToStatus(
  task: Task,
  targetStatus: DisplayStatus,
  user: CurrentUser,
): targetStatus is TaskStatus {
  if (targetStatus === "overdue") {
    return false;
  }
  return canManageTasks(user) || taskAssigneeIds(task).includes(user.id);
}

export function filterTasks(tasks: Task[], filters: TaskFilters): Task[] {
  const keyword = filters.keyword?.trim().toLowerCase();
  return tasks.filter((task) => {
    if (filters.project_id && task.project_id !== filters.project_id) return false;
    if (filters.assignee_id && !taskAssigneeIds(task).includes(filters.assignee_id)) return false;
    if (filters.status && task.status !== filters.status) return false;
    if (filters.priority && task.priority !== filters.priority) return false;
    if (keyword && !task.title.toLowerCase().includes(keyword)) return false;
    if (filters.date_from && task.due_date < filters.date_from) return false;
    if (filters.date_to && task.start_date > filters.date_to) return false;
    return true;
  });
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

export function statusLabel(status: DisplayStatus | TaskStatus): string {
  const labels: Record<DisplayStatus, string> = {
    todo: "待开始",
    in_progress: "进行中",
    done: "已完成",
    overdue: "已延期",
  };
  return labels[status];
}

export function priorityLabel(priority: Task["priority"]): string {
  return { low: "低", medium: "中", high: "高" }[priority];
}
