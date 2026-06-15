import { describe, expect, it } from "vitest";
import type { Task } from "@/types";
import {
  buildGanttTasks,
  formatGanttTooltip,
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
});
