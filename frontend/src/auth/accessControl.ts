import type { CurrentUser } from "@/types";

export function canAccessAdminModules(user: CurrentUser | undefined): boolean {
  return user?.role === "admin";
}
