import { describe, expect, it } from "vitest";
import type { DisplayStatus, Task } from "@/types";
import { visibleKanbanTasks } from "./taskKanban";

const baseTask: Task = {
  id: "task-1",
  project_id: "project-1",
  project_name: "项目管理系统",
  title: "任务",
  assignee_id: "user-1",
  assignee_name: "李四",
  assignees: [{ id: "user-1", name: "李四" }],
  status: "done",
  priority: "medium",
  start_date: "2026-06-01",
  due_date: "2026-06-10",
  description_json: {},
  creator_id: "user-2",
  updated_at: "2026-06-09T09:30:00Z",
  archived: false,
  is_overdue: false,
  display_status: "done",
};

function tasksForStatus(status: DisplayStatus, count: number): Task[] {
  return Array.from({ length: count }, (_, index) => ({
    ...baseTask,
    id: `${status}-${index}`,
    status,
    display_status: status,
  }));
}

describe("taskKanban", () => {
  it("shows only five completed tasks on the board", () => {
    expect(visibleKanbanTasks("done", tasksForStatus("done", 8))).toHaveLength(5);
  });

  it("keeps other status columns at ten visible tasks", () => {
    expect(visibleKanbanTasks("todo", tasksForStatus("todo", 12))).toHaveLength(10);
  });
});
