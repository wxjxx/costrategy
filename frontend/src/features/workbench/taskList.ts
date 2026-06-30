import type { Task, TaskSubtask } from "@/types";

export type TaskListRowKind = "task" | "subtask";

export type TaskListSubtaskRow = {
  id: string;
  subtask_id: string;
  task_id: string;
  title: string;
  project_name?: string;
  assignee_id: string;
  assignee_name?: string;
  status: TaskSubtask["status"];
  priority?: Task["priority"];
  start_date?: string;
  due_date?: string;
  updated_at: string;
  is_overdue: boolean;
  display_status: string;
  rowKind: "subtask";
};

export type TaskListParentRow = Task & {
  rowKind: "task";
  children?: TaskListSubtaskRow[];
};

export type TaskListRow = TaskListParentRow | TaskListSubtaskRow;

export function buildTaskListRows(tasks: Task[]): TaskListParentRow[] {
  return tasks.map((task) => {
    const children = (task.subtasks ?? []).map((subtask) => ({
      id: `subtask-${subtask.id}`,
      subtask_id: subtask.id,
      task_id: task.id,
      title: subtask.description,
      project_name: task.project_name,
      assignee_id: subtask.assignee_id,
      assignee_name: subtask.assignee_name,
      status: subtask.status,
      start_date: task.start_date,
      due_date: task.due_date,
      updated_at: subtask.updated_at,
      is_overdue: subtask.is_overdue,
      display_status: subtask.display_status,
      rowKind: "subtask" as const,
    }));
    return {
      ...task,
      rowKind: "task" as const,
      children: children.length ? children : undefined,
    };
  });
}
