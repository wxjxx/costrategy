import { describe, expect, it } from "vitest";
import type { CurrentUser, Task } from "@/types";
import {
  canMoveTaskToStatus,
  groupTasksByDisplayStatus,
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
});
