import type { Task } from "@/types";
import {
  statusColor,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";

export type GanttTask = {
  id: string;
  text: string;
  start_date?: string;
  end_date?: string;
  parent?: string;
  open?: boolean;
  type?: string | number;
  color?: string;
  assigneeText?: string;
  titleText?: string;
};

export function formatGanttTaskTitle(task: Task): string {
  if (!task.is_overdue) return task.title;
  return `<span class="overdue-badge gantt-overdue-badge" aria-label="已延期">延</span>${task.title}`;
}

export function buildGanttTasks(
  tasks: Task[],
  projectTaskType: string | number,
): GanttTask[] {
  const projectTasks = new Map<string, GanttTask>();
  const ganttTasks: GanttTask[] = [];

  tasks.forEach((task) => {
    const projectId = `project-${task.project_id}`;
    if (!projectTasks.has(projectId)) {
      const projectTask = {
        id: projectId,
        text: task.project_name || "未命名项目",
        type: projectTaskType,
        open: true,
      };

      projectTasks.set(projectId, projectTask);
      ganttTasks.push(projectTask);
    }

    ganttTasks.push({
      id: task.id,
      parent: projectId,
      text: formatGanttTaskTitle(task),
      start_date: task.start_date,
      end_date: task.due_date,
      color: statusColor(task.status),
      assigneeText: taskAssigneeNames(task),
      titleText: task.title,
    });
  });

  return ganttTasks;
}

export function formatGanttTooltip(task: GanttTask): string {
  if (!task.parent) return `<b>${task.text}</b>`;
  return `<b>${task.titleText || task.text}</b><br/>负责人：${task.assigneeText || "-"}`;
}

export function taskDetailPathFromGanttTask(task: GanttTask): string | undefined {
  if (!task.parent) return undefined;
  return `/tasks/${task.id}`;
}
