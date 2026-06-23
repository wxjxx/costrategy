import { describe, expect, it } from "vitest";
import type { Task } from "@/types";
import {
  buildGanttTasks,
  buildGanttTimelineRange,
  calculateCenteredTimelineScrollX,
  defaultGanttScaleMode,
  formatGanttBarContent,
  formatGanttTodayMarkerText,
  formatGanttTooltip,
  shouldToggleProjectRowFromClick,
  shouldUseDefaultGanttClick,
  taskDetailPathFromGanttTask,
} from "./taskGantt";

const task: Task = {
  id: "task-1",
  project_id: "project-1",
  project_name: "项目管理系统",
  title: "绘制甘特图",
  assignee_id: "user-1",
  assignee_name: "李四",
  assignees: [{ id: "user-1", name: "李四" }],
  status: "in_progress",
  priority: "high",
  start_date: "2026-06-01",
  due_date: "2026-06-10",
  description_json: {},
  creator_id: "user-2",
  updated_at: "2026-06-09T09:30:00Z",
  archived: false,
  is_overdue: false,
  display_status: "in_progress",
};

describe("taskGantt", () => {
  it("keeps assignee names out of task text but includes them in tooltip", () => {
    const ganttTasks = buildGanttTasks([task], "project");
    const ganttTask = ganttTasks.find((item) => item.id === task.id);

    expect(ganttTask).toMatchObject({
      id: "task-1",
      text: "绘制甘特图",
      assigneeText: "李四",
    });
    expect(formatGanttTooltip(ganttTask!)).toContain("负责人：李四");
  });

  it("defaults the timeline scale to day view", () => {
    expect(defaultGanttScaleMode).toBe("按天");
  });

  it("formats a current-date marker label for the gantt timeline", () => {
    expect(formatGanttTodayMarkerText(new Date(2026, 5, 17))).toBe(
      "今天 06月17日",
    );
  });

  it("adds enough date range padding for centering today in the timeline", () => {
    const range = buildGanttTimelineRange(
      [{ ...task, start_date: "2026-05-01", due_date: "2026-05-03" }],
      new Date(2026, 5, 17),
    );

    expect(range.start_date).toEqual(new Date(2026, 3, 3));
    expect(range.end_date).toEqual(new Date(2026, 6, 15));
  });

  it("calculates horizontal scroll so the current date is centered", () => {
    expect(calculateCenteredTimelineScrollX(700, 300)).toBe(550);
    expect(calculateCenteredTimelineScrollX(80, 300)).toBe(0);
  });

  it("lets project row clicks use the gantt default behavior for expand and collapse", () => {
    const ganttTasks = buildGanttTasks([task], "project");
    const projectRow = ganttTasks.find((item) => item.id === "project-project-1");
    const taskRow = ganttTasks.find((item) => item.id === task.id);

    expect(shouldUseDefaultGanttClick(projectRow!)).toBe(true);
    expect(shouldUseDefaultGanttClick(taskRow!)).toBe(false);
  });

  it("toggles project rows when users click the row cell instead of the tree icon", () => {
    const ganttTasks = buildGanttTasks([task], "project");
    const projectRow = ganttTasks.find((item) => item.id === "project-project-1");
    const taskRow = ganttTasks.find((item) => item.id === task.id);

    expect(shouldToggleProjectRowFromClick(projectRow!, false)).toBe(true);
    expect(shouldToggleProjectRowFromClick(projectRow!, true)).toBe(false);
    expect(shouldToggleProjectRowFromClick(taskRow!, false)).toBe(false);
  });

  it("renders a compact assignee avatar inside gantt bars", () => {
    const ganttTasks = buildGanttTasks([task], "project");
    const ganttTask = ganttTasks.find((item) => item.id === task.id);

    expect(formatGanttBarContent(ganttTask!)).toContain("gantt-task-avatar");
    expect(formatGanttBarContent(ganttTask!)).toContain("李");
    expect(formatGanttBarContent(ganttTask!)).toContain("绘制甘特图");
  });

  it("adds overdue marker to overdue task titles without changing bar color to red", () => {
    const ganttTasks = buildGanttTasks([{ ...task, is_overdue: true }], "project");
    const ganttTask = ganttTasks.find((item) => item.id === task.id);

    expect(ganttTask?.text).toContain("gantt-overdue-badge");
    expect(ganttTask?.text).toContain("延");
    expect(ganttTask?.text).toContain("绘制甘特图");
    expect(ganttTask?.text).not.toContain("李四");
    expect(ganttTask?.color).toBe("#2b7bff");
  });

  it("uses the blocked status color for blocked tasks", () => {
    const ganttTasks = buildGanttTasks([{ ...task, status: "blocked" }], "project");
    const ganttTask = ganttTasks.find((item) => item.id === task.id);

    expect(ganttTask?.color).toBe("#f97316");
  });

  it("builds detail paths for real tasks but not project rows", () => {
    const ganttTasks = buildGanttTasks([task], "project");
    const projectRow = ganttTasks.find((item) => item.id === "project-project-1");
    const taskRow = ganttTasks.find((item) => item.id === task.id);

    expect(taskDetailPathFromGanttTask(taskRow!)).toBe("/tasks/task-1");
    expect(taskDetailPathFromGanttTask(projectRow!)).toBeUndefined();
  });

  it("does not render done tasks in gantt", () => {
    const ganttTasks = buildGanttTasks(
      [task, { ...task, id: "task-2", status: "done", title: "已完成任务" }],
      "project",
    );

    expect(ganttTasks.some((item) => item.id === "task-2")).toBe(false);
  });
});
