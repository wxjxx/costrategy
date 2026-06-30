import { describe, expect, it } from "vitest";
import type { Task } from "@/types";
import { buildTaskListRows } from "./taskList";

const task: Task = {
  id: "task-1",
  project_id: "project-1",
  project_name: "项目管理系统",
  title: "父任务",
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
  is_overdue: true,
  display_status: "in_progress",
  subtasks: [
    {
      id: "subtask-1",
      task_id: "task-1",
      assignee_id: "user-1",
      assignee_name: "李四",
      status: "done",
      description: "按时完成的子任务",
      is_overdue: false,
      display_status: "done",
      updated_at: "2026-06-09T09:30:00Z",
    },
    {
      id: "subtask-2",
      task_id: "task-1",
      assignee_id: "user-2",
      assignee_name: "王五",
      status: "in_progress",
      description: "延期的子任务",
      is_overdue: true,
      display_status: "in_progress",
      updated_at: "2026-06-10T09:30:00Z",
    },
  ],
};

describe("taskList", () => {
  it("builds nested rows when a task has subtasks", () => {
    const rows = buildTaskListRows([task]);

    expect(rows[0].children).toHaveLength(2);
    expect(rows[0].children?.[0]).toMatchObject({
      id: "subtask-subtask-1",
      subtask_id: "subtask-1",
      rowKind: "subtask",
      title: "按时完成的子任务",
      is_overdue: false,
    });
    expect(rows[0].children?.[1]).toMatchObject({
      id: "subtask-subtask-2",
      rowKind: "subtask",
      title: "延期的子任务",
      is_overdue: true,
    });
  });
});
