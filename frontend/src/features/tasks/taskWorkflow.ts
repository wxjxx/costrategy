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

export function getDisplayStatus(task: Task): DisplayStatus {
  if (task.status !== "done" && task.is_overdue) {
    return "overdue";
  }
  return task.status;
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
  return canManageTasks(user) || task.assignee_id === user.id;
}

export function filterTasks(tasks: Task[], filters: TaskFilters): Task[] {
  const keyword = filters.keyword?.trim().toLowerCase();
  return tasks.filter((task) => {
    if (filters.project_id && task.project_id !== filters.project_id) return false;
    if (filters.assignee_id && task.assignee_id !== filters.assignee_id) return false;
    if (filters.status && task.status !== filters.status) return false;
    if (filters.priority && task.priority !== filters.priority) return false;
    if (keyword && !task.title.toLowerCase().includes(keyword)) return false;
    if (filters.date_from && task.due_date < filters.date_from) return false;
    if (filters.date_to && task.start_date > filters.date_to) return false;
    return true;
  });
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
