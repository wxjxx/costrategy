import type { CurrentUser } from "@/types";

let authenticationSucceeded = false;
let authenticatedUser: CurrentUser | undefined;

export function hasAuthenticationSucceeded(): boolean {
  return authenticationSucceeded;
}

export function getAuthenticatedUser(): CurrentUser | undefined {
  return authenticatedUser;
}

export function setAuthenticatedUser(currentUser: CurrentUser): void {
  authenticatedUser = currentUser;
}

export function markAuthenticationSucceeded(): void {
  authenticationSucceeded = true;
}

export function resetAuthenticationState(): void {
  authenticationSucceeded = false;
  authenticatedUser = undefined;
}
