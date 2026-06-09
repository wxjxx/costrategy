import { beforeEach, describe, expect, it, vi } from "vitest";
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

  it("redirects to the 401 page when backend returns unauthorized", async () => {
    const redirects: string[] = [];
    const error = { response: { status: 401 } };
    setUnauthorizedRedirectHandler((path) => redirects.push(path));

    await expect(redirectUnauthorizedError(error)).rejects.toBe(error);

    expect(redirects).toEqual(["/401"]);
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
});
