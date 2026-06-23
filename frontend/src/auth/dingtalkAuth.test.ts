import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  loadCurrentUserWithDingtalkLogin,
  requestDingtalkAuthCode,
  resolveAdminToken,
  resolveDingtalkClientId,
  resolveDingtalkCorpId,
} from "./dingtalkAuth";

describe("dingtalk auth", () => {
  beforeEach(() => {
    window.sessionStorage.clear();
  });

  it("reads the DingTalk clientId and corpid from the launch URL", () => {
    const locationSearch = "?clientId=ding-client&corpid=ding-corp";

    expect(resolveDingtalkClientId(locationSearch)).toBe("ding-client");
    expect(resolveDingtalkCorpId(locationSearch)).toBe("ding-corp");
  });

  it("does not accept alternate corpId parameter spellings", () => {
    expect(resolveDingtalkCorpId("?corpId=ding-corp")).toBeUndefined();
  });

  it("reuses cached DingTalk launch params after the URL query is gone", () => {
    expect(resolveDingtalkClientId("?clientId=ding-client&corpid=ding-corp")).toBe("ding-client");
    expect(resolveDingtalkCorpId("?clientId=ding-client&corpid=ding-corp")).toBe("ding-corp");

    expect(resolveDingtalkClientId("")).toBe("ding-client");
    expect(resolveDingtalkCorpId("")).toBe("ding-corp");
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
        locationSearch: "?clientId=client-id-1&corpid=ding-corp-1",
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

  it("falls back to runtime permission auth code API when the top-level API fails", async () => {
    const requestAuthCode = vi.fn().mockRejectedValue({ message: "not supported" });
    const runtimeRequestAuthCode = vi.fn().mockResolvedValue({ code: "runtime-code-1" });

    await expect(
      requestDingtalkAuthCode({
        locationSearch: "?clientId=client-id-1&corpid=ding-corp-1",
        dd: {
          requestAuthCode,
          runtime: {
            permission: {
              requestAuthCode: runtimeRequestAuthCode,
            },
          },
        },
      }),
    ).resolves.toBe("runtime-code-1");

    expect(requestAuthCode).toHaveBeenCalled();
    expect(runtimeRequestAuthCode).toHaveBeenCalledWith(
      expect.objectContaining({
        clientId: "client-id-1",
        corpId: "ding-corp-1",
      }),
    );
  });

  it("supports onSuccess callbacks from DingTalk auth code APIs", async () => {
    const requestAuthCode = vi.fn((options) => {
      options.onSuccess({ authCode: "auth-code-from-on-success" });
    });

    await expect(
      requestDingtalkAuthCode({
        locationSearch: "?clientId=client-id-1&corpid=ding-corp-1",
        dd: {
          runtime: {
            permission: {
              requestAuthCode,
            },
          },
        },
      }),
    ).resolves.toBe("auth-code-from-on-success");
  });

  it("loads the DingTalk JSAPI package when window.dd is unavailable", async () => {
    const requestAuthCode = vi.fn().mockResolvedValue({ code: "auth-code-from-package" });

    await expect(
      requestDingtalkAuthCode({
        locationSearch: "?clientId=client-id-1&corpid=ding-corp-1",
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

  it("requires clientId from the launch URL", async () => {
    const requestAuthCode = vi.fn().mockResolvedValue({ code: "auth-code-1" });

    await expect(
      requestDingtalkAuthCode({
        locationSearch: "?corpid=ding-corp-1",
        dd: { requestAuthCode },
      }),
    ).rejects.toMatchObject({ errorCode: "DINGTALK_CLIENT_ID_MISSING" });

    expect(requestAuthCode).not.toHaveBeenCalled();
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
