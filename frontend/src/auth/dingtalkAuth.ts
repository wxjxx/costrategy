import type { CurrentUser } from "@/types";

type DingtalkAuthCodeResult = { code?: string; authCode?: string };
type DingtalkAuthCodeRequest = (options: {
  clientId?: string;
  corpId: string;
  success?: (result: DingtalkAuthCodeResult) => void;
  fail?: (error: unknown) => void;
}) => void | Promise<DingtalkAuthCodeResult>;

export interface DingtalkJsApi {
  requestAuthCode?: DingtalkAuthCodeRequest;
  getAuthCode?: DingtalkAuthCodeRequest;
  runtime?: {
    permission?: {
      requestAuthCode?: DingtalkAuthCodeRequest;
    };
  };
}

interface DingtalkWindow extends Window {
  dd?: DingtalkJsApi;
}

export interface AuthBootstrapRequestOptions {
  skipUnauthorizedRedirect?: boolean;
}

export interface AuthApiClient {
  me(options?: AuthBootstrapRequestOptions): Promise<CurrentUser>;
  dingtalkLogin(code: string): Promise<CurrentUser>;
  adminTokenLogin?(token: string): Promise<CurrentUser>;
}

export class DingtalkAuthError extends Error {
  constructor(
    readonly errorCode: string,
    message: string,
    readonly detail?: unknown,
  ) {
    super(message);
    this.name = "DingtalkAuthError";
  }
}

function getCurrentSearch(): string {
  return typeof window === "undefined" ? "" : window.location.search;
}

function getDingtalkJsApi(): DingtalkJsApi | undefined {
  return typeof window === "undefined" ? undefined : (window as DingtalkWindow).dd;
}

async function loadDingtalkJsApiFromPackage(): Promise<DingtalkJsApi | undefined> {
  const imported = (await import("dingtalk-jsapi")) as unknown as {
    default?: DingtalkJsApi;
  } & DingtalkJsApi;
  return imported.default ?? imported;
}

function getErrorLogPayload(error: unknown): Record<string, unknown> {
  if (error instanceof DingtalkAuthError) {
    return {
      errorCode: error.errorCode,
      message: error.message,
      detail: error.detail,
    };
  }
  if (isUnauthorizedError(error)) {
    return { status: 401 };
  }
  if (error instanceof Error) {
    return { name: error.name, message: error.message };
  }
  return { error };
}

function resolveAuthCodeRequest(dd: DingtalkJsApi, clientId?: string) {
  if (clientId && dd.requestAuthCode) {
    return {
      request: dd.requestAuthCode.bind(dd),
      apiName: "requestAuthCode",
      includeClientId: true,
    };
  }

  const runtimePermission = dd.runtime?.permission;
  if (runtimePermission?.requestAuthCode) {
    return {
      request: runtimePermission.requestAuthCode.bind(runtimePermission),
      apiName: "runtime.permission.requestAuthCode",
      includeClientId: false,
    };
  }

  if (dd.getAuthCode) {
    return {
      request: dd.getAuthCode.bind(dd),
      apiName: "getAuthCode",
      includeClientId: false,
    };
  }

  if (dd.requestAuthCode) {
    return {
      request: dd.requestAuthCode.bind(dd),
      apiName: "requestAuthCode",
      includeClientId: Boolean(clientId),
    };
  }

  return undefined;
}

export function resolveDingtalkCorpId(
  locationSearch = getCurrentSearch(),
  fallbackCorpId = import.meta.env.VITE_DINGTALK_CORP_ID,
): string | undefined {
  const params = new URLSearchParams(locationSearch);
  const corpId = params.get("corpid") ?? params.get("corpId") ?? fallbackCorpId;
  return corpId || undefined;
}

export function resolveAdminToken(locationSearch = getCurrentSearch()): string | undefined {
  const token = new URLSearchParams(locationSearch).get("admin-token");
  return token || undefined;
}

export function isUnauthorizedError(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "response" in error &&
    (error as { response?: { status?: number } }).response?.status === 401
  );
}

export async function requestDingtalkAuthCode(options: {
  clientId?: string;
  corpId?: string;
  dd?: DingtalkJsApi;
  loadDd?: () => Promise<DingtalkJsApi | undefined>;
  locationSearch?: string;
} = {}): Promise<string> {
  let dd = options.dd ?? getDingtalkJsApi();
  if (!dd) {
    console.info("[auth:dingtalk] DingTalk JSAPI not found on window, loading npm package");
    dd = await (options.loadDd ?? loadDingtalkJsApiFromPackage)();
  }
  const clientId = options.clientId ?? import.meta.env.VITE_DINGTALK_CLIENT_ID;
  const corpId = options.corpId ?? resolveDingtalkCorpId(options.locationSearch);
  const corpIdSource =
    options.corpId !== undefined
      ? "argument"
      : new URLSearchParams(options.locationSearch ?? getCurrentSearch()).has("corpid") ||
          new URLSearchParams(options.locationSearch ?? getCurrentSearch()).has("corpId")
        ? "url"
        : import.meta.env.VITE_DINGTALK_CORP_ID
          ? "env"
          : "missing";

  if (!dd) {
    console.warn("[auth:dingtalk] DingTalk JSAPI is missing");
    throw new DingtalkAuthError("DINGTALK_JSAPI_MISSING", "当前环境不支持钉钉免登");
  }
  if (!corpId) {
    console.warn("[auth:dingtalk] DingTalk corpId is missing");
    throw new DingtalkAuthError("DINGTALK_CORP_ID_MISSING", "缺少钉钉企业 CorpId");
  }
  const authCodeRequest = resolveAuthCodeRequest(dd, clientId);
  if (!authCodeRequest) {
    console.warn("[auth:dingtalk] DingTalk auth code API is missing");
    throw new DingtalkAuthError("DINGTALK_JSAPI_MISSING", "当前环境不支持钉钉免登");
  }
  if (!clientId && authCodeRequest.includeClientId) {
    console.warn("[auth:dingtalk] DingTalk clientId is missing");
    throw new DingtalkAuthError("DINGTALK_CLIENT_ID_MISSING", "缺少钉钉应用 Client ID");
  }

  console.info("[auth:dingtalk] requesting DingTalk auth code", {
    corpIdSource,
    hasClientId: Boolean(clientId),
    apiName: authCodeRequest.apiName,
  });

  return new Promise((resolve, reject) => {
    let settled = false;
    const onSuccess = (result: DingtalkAuthCodeResult) => {
      if (settled) {
        return;
      }
      const code = result.code ?? result.authCode;
      if (code) {
        settled = true;
        console.info("[auth:dingtalk] DingTalk auth code received");
        resolve(code);
        return;
      }
      settled = true;
      console.warn("[auth:dingtalk] DingTalk auth code response is empty", result);
      reject(
        new DingtalkAuthError(
          "DINGTALK_AUTH_CODE_EMPTY",
          "钉钉免登未返回授权码",
          result,
        ),
      );
    };
    const onFail = (error: unknown) => {
      if (settled) {
        return;
      }
      settled = true;
      console.error("[auth:dingtalk] DingTalk auth code request failed", error);
      reject(
        new DingtalkAuthError(
          "DINGTALK_AUTH_CODE_FAILED",
          "获取钉钉免登授权码失败",
          error,
        ),
      );
    };

    const result = authCodeRequest.request({
      ...(authCodeRequest.includeClientId ? { clientId } : {}),
      corpId,
      success: onSuccess,
      fail: onFail,
    });

    if (result && "then" in result) {
      result.then(onSuccess).catch(onFail);
    }
  });
}

export async function loadCurrentUserWithDingtalkLogin(options: {
  authApi: AuthApiClient;
  requestAuthCode?: () => Promise<string>;
  redirectUnauthorized?: () => void | Promise<void>;
}): Promise<CurrentUser> {
  const requestOptions = { skipUnauthorizedRedirect: true };

  console.info("[auth:dingtalk] checking existing backend session");
  try {
    const currentUser = await options.authApi.me(requestOptions);
    console.info("[auth:dingtalk] existing backend session is valid", {
      userId: currentUser.id,
      role: currentUser.role,
    });
    return currentUser;
  } catch (error) {
    if (!isUnauthorizedError(error)) {
      console.error(
        "[auth:dingtalk] current user request failed before DingTalk login",
        getErrorLogPayload(error),
      );
      throw error;
    }
    console.info("[auth:dingtalk] backend session is missing, starting DingTalk login");
  }

  try {
    const code = await (options.requestAuthCode ?? requestDingtalkAuthCode)();
    console.info("[auth:dingtalk] exchanging DingTalk auth code with backend");
    await options.authApi.dingtalkLogin(code);
    console.info("[auth:dingtalk] backend DingTalk login succeeded, reloading current user");
    const currentUser = await options.authApi.me(requestOptions);
    console.info("[auth:dingtalk] current user loaded after DingTalk login", {
      userId: currentUser.id,
      role: currentUser.role,
    });
    return currentUser;
  } catch (error) {
    console.error("[auth:dingtalk] DingTalk login flow failed", getErrorLogPayload(error));
    await options.redirectUnauthorized?.();
    throw error;
  }
}
