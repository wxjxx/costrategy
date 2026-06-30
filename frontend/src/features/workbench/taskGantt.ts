import type { Task } from "@/types";
import {
  primaryTaskAssigneeName,
  statusColor,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";

export type GanttScaleMode = "按天" | "按周";

export const defaultGanttScaleMode: GanttScaleMode = "按天";
const GANTT_TIMELINE_PADDING_DAYS = 28;

export type GanttTask = {
  id: string;
  text: string;
  start_date?: string;
  end_date?: string;
  parent?: string;
  open?: boolean;
  $open?: boolean;
  type?: string | number;
  color?: string;
  assigneeText?: string;
  avatarText?: string;
  titleText?: string;
  rowKind?: "project" | "task" | "subtask";
  hasChildren?: boolean;
};

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function parseLocalDate(value: string | undefined): Date | undefined {
  if (!value) return undefined;
  const [year, month, day] = value.split("-").map(Number);
  if (!year || !month || !day) return undefined;
  return new Date(year, month - 1, day);
}

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function addDays(date: Date, days: number): Date {
  const nextDate = new Date(date);
  nextDate.setDate(date.getDate() + days);
  return nextDate;
}

export function formatGanttTodayMarkerText(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `今天 ${month}月${day}日`;
}

export function buildGanttTimelineRange(
  tasks: Task[],
  today: Date,
): { start_date: Date; end_date: Date } {
  const todayStart = startOfLocalDay(today);
  const taskDates = tasks.flatMap((task) =>
    [parseLocalDate(task.start_date), parseLocalDate(task.due_date)].filter(
      (date): date is Date => Boolean(date),
    ),
  );
  const allDates = [todayStart, ...taskDates];
  const minTime = Math.min(...allDates.map((date) => date.getTime()));
  const maxTime = Math.max(...allDates.map((date) => date.getTime()));

  return {
    start_date: addDays(new Date(minTime), -GANTT_TIMELINE_PADDING_DAYS),
    end_date: addDays(new Date(maxTime), GANTT_TIMELINE_PADDING_DAYS),
  };
}

export function calculateCenteredTimelineScrollX(
  datePositionX: number,
  viewportWidth: number,
): number {
  return Math.max(0, Math.round(datePositionX - viewportWidth / 2));
}

export function formatGanttTaskTitle(task: Task): string {
  const title = escapeHtml(task.title);
  if (!task.is_overdue) return title;
  return `<span class="overdue-badge gantt-overdue-badge" aria-label="已延期">延</span>${title}`;
}

export function formatGanttBarContent(task: GanttTask): string {
  if (!task.parent) return task.text;
  const avatarText = escapeHtml(task.avatarText || "用");
  return `<span class="gantt-task-bar-content"><span class="user-avatar gantt-task-avatar">${avatarText}</span><span class="gantt-task-bar-title">${task.text}</span></span>`;
}

export function shouldUseDefaultGanttClick(task: GanttTask): boolean {
  return task.rowKind === "project" || Boolean(task.hasChildren);
}

export function shouldToggleProjectRowFromClick(
  task: GanttTask,
  clickedTreeIcon: boolean,
): boolean {
  return (task.rowKind === "project" || Boolean(task.hasChildren)) && !clickedTreeIcon;
}

export function isGanttProjectOpen(task: GanttTask): boolean {
  return Boolean(task.$open ?? task.open);
}

export function buildGanttTasks(
  tasks: Task[],
  projectTaskType: string | number,
): GanttTask[] {
  const projectTasks = new Map<string, GanttTask>();
  const ganttTasks: GanttTask[] = [];

  tasks.filter((task) => task.status !== "done").forEach((task) => {
    const projectId = `project-${task.project_id}`;
    if (!projectTasks.has(projectId)) {
      const projectTask = {
        id: projectId,
        text: escapeHtml(task.project_name || "未命名项目"),
        type: projectTaskType,
        open: true,
        rowKind: "project" as const,
        hasChildren: true,
      };

      projectTasks.set(projectId, projectTask);
      ganttTasks.push(projectTask);
    }

    const subtasks = task.subtasks ?? [];
    ganttTasks.push({
      id: task.id,
      parent: projectId,
      text: formatGanttTaskTitle(task),
      start_date: task.start_date,
      end_date: task.due_date,
      color: statusColor(task.status),
      assigneeText: taskAssigneeNames(task),
      avatarText: (primaryTaskAssigneeName(task) || "用").slice(0, 1),
      titleText: task.title,
      open: subtasks.length > 0,
      rowKind: "task",
      hasChildren: subtasks.length > 0,
    });

    subtasks.forEach((subtask) => {
      ganttTasks.push({
        id: `subtask-${subtask.id}`,
        parent: task.id,
        text: subtask.is_overdue
          ? `<span class="overdue-badge gantt-overdue-badge" aria-label="已延期">延</span>${escapeHtml(subtask.description)}`
          : escapeHtml(subtask.description),
        start_date: task.start_date,
        end_date: task.due_date,
        color: statusColor(subtask.status),
        assigneeText: subtask.assignee_name || "-",
        avatarText: (subtask.assignee_name || "用").slice(0, 1),
        titleText: subtask.description,
        rowKind: "subtask",
      });
    });
  });

  return ganttTasks;
}

export function formatGanttTooltip(task: GanttTask): string {
  if (!task.parent) return `<b>${task.text}</b>`;
  const title = escapeHtml(task.titleText || task.text);
  const assignee = escapeHtml(task.assigneeText || "-");
  return `<b>${title}</b><br/>负责人：${assignee}`;
}

export function taskDetailPathFromGanttTask(task: GanttTask): string | undefined {
  if (task.rowKind !== "task" || task.hasChildren) return undefined;
  return `/tasks/${task.id}`;
}
