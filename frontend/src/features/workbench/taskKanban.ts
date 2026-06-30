import type { DisplayStatus, Task } from "@/types";

export function visibleKanbanTasks(status: DisplayStatus, tasks: Task[]): Task[] {
  return tasks.slice(0, status === "done" ? 5 : 10);
}

export function kanbanStatusLimit(status: DisplayStatus): number {
  return status === "done" ? 5 : 10;
}
