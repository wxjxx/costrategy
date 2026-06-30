import { describe, expect, it } from "vitest";
import projectsPage from "@/features/projects/ProjectsPage.vue?raw";
import taskDetailPage from "@/features/tasks/TaskDetailPage.vue?raw";
import usersPage from "@/features/users/UsersPage.vue?raw";
import taskKanbanView from "@/features/workbench/TaskKanbanView.vue?raw";
import taskListView from "@/features/workbench/TaskListView.vue?raw";

describe("user avatar bindings", () => {
  it("passes avatar urls to user avatars outside the app header", () => {
    expect(usersPage).toContain(':src="row.avatar_url"');
    expect(projectsPage).toContain(':src="ownerAvatar(row.owner_id)"');
    expect(taskDetailPage).toContain(':src="primaryTaskAssigneeAvatar(task)"');
    expect(taskDetailPage).toContain(':src="userAvatar(row.assignee_id)"');
    expect(taskKanbanView).toContain(':src="primaryTaskAssigneeAvatar(task)"');
    expect(taskListView).toContain(':src="rowPrimaryAssigneeAvatar(row)"');
  });
});
