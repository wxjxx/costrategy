import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getAuthenticatedUser,
  hasAuthenticationSucceeded,
  markAuthenticationSucceeded,
  setAuthenticatedUser,
} from "@/auth/sessionState";
import { api, http, redirectUnauthorizedError, setUnauthorizedRedirectHandler } from "./client";

describe("api client", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    setUnauthorizedRedirectHandler(undefined);
  });

  it("propagates backend errors instead of returning sample data", async () => {
    const error = new Error("backend unavailable");
    vi.spyOn(http, "get").mockRejectedValue(error);

    await expect(api.tasks()).rejects.toBe(error);
  });

  it("clears cached authentication and returns home when backend session expires", async () => {
    const redirects: string[] = [];
    setAuthenticatedUser({
      id: "user-1",
      name: "张三",
      role: "employee",
      departments: [],
      permissions: [],
    });
    markAuthenticationSucceeded();
    const error = { response: { status: 401 } };
    setUnauthorizedRedirectHandler((path) => redirects.push(path));

    await expect(redirectUnauthorizedError(error)).rejects.toBe(error);

    expect(redirects).toEqual(["/"]);
    expect(hasAuthenticationSucceeded()).toBe(false);
    expect(getAuthenticatedUser()).toBeUndefined();
  });

  it("can skip the automatic 401 redirect for auth bootstrap requests", async () => {
    const redirects: string[] = [];
    const error = { response: { status: 401 }, config: { skipUnauthorizedRedirect: true } };
    setUnauthorizedRedirectHandler((path) => redirects.push(path));

    await expect(redirectUnauthorizedError(error)).rejects.toBe(error);

    expect(redirects).toEqual([]);
  });

  it("exchanges a DingTalk auth code with the backend login endpoint", async () => {
    const currentUser = {
      id: "user-1",
      name: "张三",
      role: "employee" as const,
      departments: [],
      permissions: [],
    };
    const post = vi.spyOn(http, "post").mockResolvedValue({ data: currentUser });

    await expect(api.dingtalkLogin("auth-code-1")).resolves.toBe(currentUser);

    expect(post).toHaveBeenCalledWith("/auth/dingtalk/login", { code: "auth-code-1" });
  });

  it("exchanges an admin auth token with the backend login endpoint", async () => {
    const currentUser = {
      id: "admin-user",
      name: "系统管理员",
      role: "admin" as const,
      departments: [],
      permissions: ["run_dingtalk_sync"],
    };
    const post = vi.spyOn(http, "post").mockResolvedValue({ data: currentUser });

    await expect(api.adminTokenLogin("admin-token-1")).resolves.toBe(currentUser);

    expect(post).toHaveBeenCalledWith("/auth/admin-token/login", {
      token: "admin-token-1",
    });
  });

  it("sends complete project create and update payloads to backend routes", async () => {
    const project = {
      id: "project-1",
      code: "PRJ-001",
      name: "项目管理系统",
      owner_id: "user-1",
      description: "内部协作系统",
      start_date: "2026-06-01",
      end_date: "2026-06-30",
      status: "active" as const,
    };
    const post = vi.spyOn(http, "post").mockResolvedValueOnce({ data: project });
    const put = vi.spyOn(http, "put").mockResolvedValueOnce({ data: project });

    await expect(
      api.createProject({
        code: "PRJ-001",
        name: "项目管理系统",
        owner_id: "user-1",
        description: "内部协作系统",
        start_date: "2026-06-01",
        end_date: "2026-06-30",
        status: "active",
      }),
    ).resolves.toBe(project);
    await expect(
      api.updateProject("project-1", {
        name: "项目管理系统",
        owner_id: "user-1",
        description: "内部协作系统",
        start_date: "2026-06-01",
        end_date: "2026-06-30",
        status: "completed",
      }),
    ).resolves.toBe(project);

    expect(post).toHaveBeenCalledWith("/projects", {
      code: "PRJ-001",
      name: "项目管理系统",
      owner_id: "user-1",
      description: "内部协作系统",
      start_date: "2026-06-01",
      end_date: "2026-06-30",
      status: "active",
    });
    expect(put).toHaveBeenCalledWith("/projects/project-1", {
      name: "项目管理系统",
      owner_id: "user-1",
      description: "内部协作系统",
      start_date: "2026-06-01",
      end_date: "2026-06-30",
      status: "completed",
    });
  });

  it("calls project and task delete endpoints", async () => {
    const project = { id: "project-1", status: "archived" };
    const task = { id: "task-1", archived: true };
    const del = vi
      .spyOn(http, "delete")
      .mockResolvedValueOnce({ data: project })
      .mockResolvedValueOnce({ data: task });

    await expect(api.deleteProject("project-1")).resolves.toBe(project);
    await expect(api.deleteTask("task-1")).resolves.toBe(task);

    expect(del).toHaveBeenNthCalledWith(1, "/projects/project-1");
    expect(del).toHaveBeenNthCalledWith(2, "/tasks/task-1");
  });

  it("updates settings with the backend batch payload", async () => {
    const response = { settings: [], connection_status: { dingtalk: "not_checked", rustfs: "configured" } };
    const put = vi.spyOn(http, "put").mockResolvedValue({ data: response });

    await expect(
      api.updateSettings([
        { key: "dingtalk.corp_id", value: "corp-1" },
        { key: "rustfs.bucket", value: "costrategy" },
      ]),
    ).resolves.toBe(response);

    expect(put).toHaveBeenCalledWith("/settings", {
      settings: [
        { key: "dingtalk.corp_id", value: "corp-1" },
        { key: "rustfs.bucket", value: "costrategy" },
      ],
    });
  });

  it("calls task collaboration attachment and comment endpoints", async () => {
    const comment = {
      id: "comment-1",
      task_id: "task-1",
      author_id: "user-1",
      content: "已完成联调",
      created_at: "2026-06-10T08:00:00Z",
    };
    const attachment = {
      id: "attachment-1",
      task_id: "task-1",
      file_name: "需求说明.txt",
      file_size: 128,
      uploader_id: "user-1",
      created_at: "2026-06-10T08:00:00Z",
    };
    const blob = new Blob(["file-content"], { type: "text/plain" });
    const post = vi
      .spyOn(http, "post")
      .mockResolvedValueOnce({ data: comment })
      .mockResolvedValueOnce({ data: attachment });
    const get = vi.spyOn(http, "get").mockResolvedValueOnce({ data: blob });
    const del = vi.spyOn(http, "delete").mockResolvedValueOnce({ data: attachment });
    const file = new File(["file-content"], "需求说明.txt", { type: "text/plain" });

    await expect(api.createTaskComment("task-1", "已完成联调")).resolves.toBe(comment);
    await expect(api.uploadTaskAttachment("task-1", file)).resolves.toBe(attachment);
    await expect(api.downloadTaskAttachment("task-1", "attachment-1")).resolves.toBe(blob);
    await expect(api.deleteTaskAttachment("task-1", "attachment-1")).resolves.toBe(attachment);

    expect(post).toHaveBeenNthCalledWith(1, "/tasks/task-1/comments", {
      content: "已完成联调",
    });
    expect(post.mock.calls[1]?.[0]).toBe("/tasks/task-1/attachments");
    expect(post.mock.calls[1]?.[1]).toBeInstanceOf(FormData);
    expect(get).toHaveBeenCalledWith("/tasks/task-1/attachments/attachment-1/download", {
      responseType: "blob",
    });
    expect(del).toHaveBeenCalledWith("/tasks/task-1/attachments/attachment-1");
  });

  it("uploads rich text images through a standalone RustFS endpoint", async () => {
    const upload = { url: "/api/rich-text/images/image-1.png" };
    const post = vi.spyOn(http, "post").mockResolvedValueOnce({ data: upload });
    const file = new File(["image"], "截图.png", { type: "image/png" });

    await expect(api.uploadRichTextImage(file)).resolves.toBe(upload);

    expect(post.mock.calls[0]?.[0]).toBe("/rich-text/images");
    expect(post.mock.calls[0]?.[1]).toBeInstanceOf(FormData);
  });

  it("calls current user notification list and read endpoints", async () => {
    const notification = {
      id: "notice-1",
      notification_type: "task_assigned",
      receiver_id: "user-1",
      task_id: "task-1",
      jump_url: "/tasks/task-1",
      content_summary: "新任务分配",
      status: "success",
      sent_at: "2026-06-10T08:00:00Z",
    };
    const get = vi.spyOn(http, "get").mockResolvedValueOnce({ data: [notification] });
    const patch = vi
      .spyOn(http, "patch")
      .mockResolvedValueOnce({ data: { ...notification, read_at: "2026-06-10T09:00:00Z" } });

    await expect(api.myNotifications()).resolves.toEqual([notification]);
    await expect(api.markMyNotificationRead("notice-1")).resolves.toMatchObject({
      id: "notice-1",
      read_at: "2026-06-10T09:00:00Z",
    });

    expect(get).toHaveBeenCalledWith("/my-notifications");
    expect(patch).toHaveBeenCalledWith("/my-notifications/notice-1/read");
  });
});
