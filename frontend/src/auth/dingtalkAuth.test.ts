import { describe, expect, it, vi } from "vitest";
import {
  loadCurrentUserWithDingtalkLogin,
  requestDingtalkAuthCode,
  resolveAdminToken,
  resolveDingtalkCorpId,
} from "./dingtalkAuth";

describe("dingtalk auth", () => {
  it("uses the DingTalk corpId from the launch URL before the env fallback", () => {
    expect(
      resolveDingtalkCorpId("?corpid=ding-from-url", "ding-from-env"),
    ).toBe("ding-from-url");
  });

  it("reads the admin auth token from the launch URL", () => {
    expect(resolveAdminToken("?admin-token=admin-token-1&corpid=ding-corp")).toBe(
      "admin-token-1",
    );
  });

  it("requests a DingTalk micro-app auth code with clientId and corpId", async () => {
    const requestAuthCode = vi.fn((options) => {
      options.success({ code: "auth-code-1" });
    });

    await expect(
      requestDingtalkAuthCode({
        clientId: "client-id-1",
        corpId: "ding-corp-1",
        dd: { requestAuthCode },
      }),
    ).resolves.toBe("auth-code-1");

    expect(requestAuthCode).toHaveBeenCalledWith(
      expect.objectContaining({
        clientId: "client-id-1",
        corpId: "ding-corp-1",
      }),
    );
  });

  it("loads the DingTalk JSAPI package when window.dd is unavailable", async () => {
    const requestAuthCode = vi.fn().mockResolvedValue({ code: "auth-code-from-package" });

    await expect(
      requestDingtalkAuthCode({
        clientId: "client-id-1",
        corpId: "ding-corp-1",
        loadDd: () => Promise.resolve({ requestAuthCode }),
      }),
    ).resolves.toBe("auth-code-from-package");

    expect(requestAuthCode).toHaveBeenCalledWith(
      expect.objectContaining({
        clientId: "client-id-1",
        corpId: "ding-corp-1",
      }),
    );
  });

  it("exchanges a DingTalk auth code when the current user request is unauthorized", async () => {
    const currentUser = {
      id: "user-1",
      name: "张三",
      role: "employee" as const,
      departments: [],
      permissions: [],
    };
    const me = vi
      .fn()
      .mockRejectedValueOnce({ response: { status: 401 } })
      .mockResolvedValueOnce(currentUser);
    const dingtalkLogin = vi.fn().mockResolvedValue(currentUser);

    await expect(
      loadCurrentUserWithDingtalkLogin({
        authApi: { me, dingtalkLogin },
        requestAuthCode: () => Promise.resolve("auth-code-1"),
      }),
    ).resolves.toBe(currentUser);

    expect(me).toHaveBeenNthCalledWith(1, { skipUnauthorizedRedirect: true });
    expect(dingtalkLogin).toHaveBeenCalledWith("auth-code-1");
    expect(me).toHaveBeenNthCalledWith(2, { skipUnauthorizedRedirect: true });
  });
});
