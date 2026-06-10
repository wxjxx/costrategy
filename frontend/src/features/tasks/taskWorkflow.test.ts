import { describe, expect, it } from "vitest";
import type { CurrentUser, Task } from "@/types";
import {
  canMoveTaskToStatus,
  filterTasks,
  groupTasksByDisplayStatus,
  moveTaskForDisplay,
  statusColor,
  taskDisplayStatusColor,
} from "./taskWorkflow";

const baseTask: Task = {
  id: "task-1",
  project_id: "project-1",
  project_name: "项目管理系统升级",
  title: "搭建甘特图页面",
  assignee_id: "user-1",
  assignee_name: "李四",
  status: "in_progress",
  priority: "high",
  start_date: "2025-05-18",
  due_date: "2025-05-30",
  description_json: {},
  creator_id: "user-2",
  archived: false,
  is_overdue: false,
  display_status: "进行中",
};

const employee: CurrentUser = {
  id: "user-1",
  name: "李四",
  role: "employee",
  departments: ["项目管理部"],
  permissions: [],
};

describe("taskWorkflow", () => {
  it("groups unfinished overdue tasks into the overdue display column", () => {
    const grouped = groupTasksByDisplayStatus([
      baseTask,
      { ...baseTask, id: "task-2", status: "todo", is_overdue: true },
      { ...baseTask, id: "task-3", status: "done", is_overdue: true },
    ]);

    expect(grouped.in_progress.map((task) => task.id)).toEqual(["task-1"]);
    expect(grouped.overdue.map((task) => task.id)).toEqual(["task-2"]);
    expect(grouped.done.map((task) => task.id)).toEqual(["task-3"]);
  });

  it("allows employees to move only their own task to real statuses", () => {
    expect(canMoveTaskToStatus(baseTask, "done", employee)).toBe(true);
    expect(
      canMoveTaskToStatus(
        { ...baseTask, assignee_id: "user-9" },
        "done",
        employee,
      ),
    ).toBe(false);
    expect(canMoveTaskToStatus(baseTask, "overdue", employee)).toBe(false);
    expect(
      canMoveTaskToStatus(baseTask, "done", { ...employee, role: "manager" }),
    ).toBe(true);
  });

  it("matches any task assignee for permissions and filters", () => {
    const task = {
      ...baseTask,
      assignee_id: "user-9",
      assignees: [
        { id: "user-9", name: "王五" },
        { id: employee.id, name: employee.name },
      ],
    };

    expect(canMoveTaskToStatus(task, "done", employee)).toBe(true);
    expect(filterTasks([task], { assignee_id: employee.id })).toEqual([task]);
  });

  it("updates a task locally for optimistic kanban display", () => {
    const moved = moveTaskForDisplay(
      [{ ...baseTask, status: "todo", is_overdue: true }],
      "task-1",
      "in_progress",
    );

    expect(moved[0]).toMatchObject({
      id: "task-1",
      status: "in_progress",
      is_overdue: false,
    });
    expect(groupTasksByDisplayStatus(moved).in_progress.map((task) => task.id)).toEqual([
      "task-1",
    ]);
  });

  it("uses one color palette for task status displays", () => {
    expect(statusColor("todo")).toBe("#8b95a5");
    expect(statusColor("in_progress")).toBe("#2b7bff");
    expect(statusColor("done")).toBe("#35b86b");
    expect(statusColor("overdue")).toBe("#ff4d4f");
    expect(
      taskDisplayStatusColor({ ...baseTask, status: "todo", is_overdue: true }),
    ).toBe("#ff4d4f");
  });
});
