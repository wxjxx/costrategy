import type { CurrentUser } from "@/types";

type DingtalkAuthCodeResult = { code?: string; authCode?: string };
type DingtalkAuthCodeRequest = (options: {
  clientId: string;
  corpId: string;
  success?: (result: DingtalkAuthCodeResult) => void;
  fail?: (error: unknown) => void;
  onSuccess?: (result: DingtalkAuthCodeResult) => void;
  onFail?: (error: unknown) => void;
}) => void | Promise<DingtalkAuthCodeResult>;

export interface DingtalkJsApi {
  requestAuthCode?: DingtalkAuthCodeRequest;
  runtime?: {
    permission?: {
      requestAuthCode?: DingtalkAuthCodeRequest;
    };
  };
  biz?: {
    auth?: {
      requestAuthCode?: DingtalkAuthCodeRequest;
    };
  };
  channel?: {
    permission?: {
      requestAuthCode?: DingtalkAuthCodeRequest;
    };
  };
}

interface DingtalkWindow extends Window {
  dd?: DingtalkJsApi;
}

const DINGTALK_LAUNCH_PARAMS_STORAGE_KEY = "costrategy:dingtalk-launch-params";

type DingtalkLaunchParams = {
  clientId?: string;
  corpId?: string;
};

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

function getLaunchParamStorage(): Storage | undefined {
  if (typeof window === "undefined") return undefined;
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

function readCachedDingtalkLaunchParams(): DingtalkLaunchParams {
  const storage = getLaunchParamStorage();
  if (!storage) return {};
  const raw = storage.getItem(DINGTALK_LAUNCH_PARAMS_STORAGE_KEY);
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as DingtalkLaunchParams;
    return {
      clientId: typeof parsed.clientId === "string" ? parsed.clientId : undefined,
      corpId: typeof parsed.corpId === "string" ? parsed.corpId : undefined,
    };
  } catch {
    storage.removeItem(DINGTALK_LAUNCH_PARAMS_STORAGE_KEY);
    return {};
  }
}

function cacheDingtalkLaunchParams(params: DingtalkLaunchParams) {
  if (!params.clientId && !params.corpId) return;
  const storage = getLaunchParamStorage();
  if (!storage) return;
  storage.setItem(DINGTALK_LAUNCH_PARAMS_STORAGE_KEY, JSON.stringify(params));
}

export function cacheDingtalkLaunchParamsFromSearch(
  locationSearch = getCurrentSearch(),
): DingtalkLaunchParams {
  const params = new URLSearchParams(locationSearch);
  const clientId = params.get("clientId") || undefined;
  const corpId = params.get("corpid") || params.get("corpId") || undefined;
  const cached = readCachedDingtalkLaunchParams();
  const nextParams = {
    clientId: clientId ?? cached.clientId,
    corpId: corpId ?? cached.corpId,
  };

  cacheDingtalkLaunchParams(nextParams);
  return nextParams;
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

export function resolveDingtalkClientId(locationSearch = getCurrentSearch()): string | undefined {
  return cacheDingtalkLaunchParamsFromSearch(locationSearch).clientId;
}

export function resolveDingtalkCorpId(locationSearch = getCurrentSearch()): string | undefined {
  return cacheDingtalkLaunchParamsFromSearch(locationSearch).corpId;
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

function authCodeCandidates(dd: DingtalkJsApi): Array<{
  name: string;
  request: DingtalkAuthCodeRequest;
}> {
  return [
    { name: "requestAuthCode", request: dd.requestAuthCode },
    {
      name: "runtime.permission.requestAuthCode",
      request: dd.runtime?.permission?.requestAuthCode,
    },
    { name: "biz.auth.requestAuthCode", request: dd.biz?.auth?.requestAuthCode },
    {
      name: "channel.permission.requestAuthCode",
      request: dd.channel?.permission?.requestAuthCode,
    },
  ].filter(
    (candidate): candidate is { name: string; request: DingtalkAuthCodeRequest } =>
      typeof candidate.request === "function",
  );
}

function extractDingtalkAuthCode(result: DingtalkAuthCodeResult): string | undefined {
  return result.code ?? result.authCode;
}

function requestAuthCodeWithCandidate(
  candidate: { name: string; request: DingtalkAuthCodeRequest },
  params: { clientId: string; corpId: string },
): Promise<string> {
  return new Promise((resolve, reject) => {
    let settled = false;
    const timeout = window.setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(
        new DingtalkAuthError(
          "DINGTALK_AUTH_CODE_TIMEOUT",
          "获取钉钉免登授权码超时",
          { apiName: candidate.name },
        ),
      );
    }, 10_000);
    const finish = (callback: () => void) => {
      if (settled) return;
      settled = true;
      window.clearTimeout(timeout);
      callback();
    };
    const onSuccess = (result: DingtalkAuthCodeResult) => {
      const code = extractDingtalkAuthCode(result);
      if (code) {
        finish(() => resolve(code));
        return;
      }
      finish(() =>
        reject(
          new DingtalkAuthError(
            "DINGTALK_AUTH_CODE_EMPTY",
            "钉钉免登未返回授权码",
            { apiName: candidate.name, result },
          ),
        ),
      );
    };
    const onFail = (error: unknown) => {
      finish(() =>
        reject(
          new DingtalkAuthError(
            "DINGTALK_AUTH_CODE_FAILED",
            "获取钉钉免登授权码失败",
            { apiName: candidate.name, error },
          ),
        ),
      );
    };

    try {
      const result = candidate.request({
        clientId: params.clientId,
        corpId: params.corpId,
        success: onSuccess,
        fail: onFail,
        onSuccess,
        onFail,
      });

      if (result && "then" in result) {
        result.then(onSuccess).catch(onFail);
      }
    } catch (error) {
      onFail(error);
    }
  });
}

export async function requestDingtalkAuthCode(options: {
  dd?: DingtalkJsApi;
  loadDd?: () => Promise<DingtalkJsApi | undefined>;
  locationSearch?: string;
} = {}): Promise<string> {
  let dd = options.dd ?? getDingtalkJsApi();
  if (!dd) {
    console.info("[auth:dingtalk] DingTalk JSAPI not found on window, loading npm package");
    dd = await (options.loadDd ?? loadDingtalkJsApiFromPackage)();
  }
  const locationSearch = options.locationSearch ?? getCurrentSearch();
  const clientId = resolveDingtalkClientId(locationSearch);
  const corpId = resolveDingtalkCorpId(locationSearch);
  const candidates = dd ? authCodeCandidates(dd) : [];

  if (!dd || candidates.length === 0) {
    console.warn("[auth:dingtalk] DingTalk JSAPI is missing");
    throw new DingtalkAuthError("DINGTALK_JSAPI_MISSING", "当前环境不支持钉钉免登");
  }
  if (!clientId) {
    console.warn("[auth:dingtalk] DingTalk clientId is missing from URL");
    throw new DingtalkAuthError("DINGTALK_CLIENT_ID_MISSING", "缺少钉钉应用 Client ID");
  }
  if (!corpId) {
    console.warn("[auth:dingtalk] DingTalk corpId is missing from URL");
    throw new DingtalkAuthError("DINGTALK_CORP_ID_MISSING", "缺少钉钉企业 CorpId");
  }

  console.info("[auth:dingtalk] requesting DingTalk auth code", {
    hasClientId: true,
    apiNames: candidates.map((candidate) => candidate.name),
  });

  const errors: unknown[] = [];
  for (const candidate of candidates) {
    try {
      const code = await requestAuthCodeWithCandidate(candidate, { clientId, corpId });
      console.info("[auth:dingtalk] DingTalk auth code received", {
        apiName: candidate.name,
      });
      return code;
    } catch (error) {
      errors.push(error);
      console.warn("[auth:dingtalk] DingTalk auth code candidate failed", {
        apiName: candidate.name,
        error: getErrorLogPayload(error),
      });
    }
  }

  console.error("[auth:dingtalk] all DingTalk auth code candidates failed", {
    apiNames: candidates.map((candidate) => candidate.name),
    errors: errors.map(getErrorLogPayload),
  });
  throw new DingtalkAuthError(
    "DINGTALK_AUTH_CODE_FAILED",
    "获取钉钉免登授权码失败",
    { apiNames: candidates.map((candidate) => candidate.name), errors },
  );
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
