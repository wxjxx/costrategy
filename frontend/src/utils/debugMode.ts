import VConsole from "vconsole";

const DEBUG_STORAGE_KEY = "debug";

let vConsole: VConsole | undefined;

export function isDebugModeEnabled(): boolean {
  return localStorage.getItem(DEBUG_STORAGE_KEY) === "1";
}

export function enableDebugMode() {
  localStorage.setItem(DEBUG_STORAGE_KEY, "1");
  ensureVConsole();
}

export function disableDebugMode() {
  localStorage.removeItem(DEBUG_STORAGE_KEY);
  vConsole?.destroy();
  vConsole = undefined;
}

export function setDebugMode(enabled: boolean) {
  if (enabled) {
    enableDebugMode();
  } else {
    disableDebugMode();
  }
}

export function ensureVConsole() {
  if (!isDebugModeEnabled() || vConsole) return;
  vConsole = new VConsole();
}
