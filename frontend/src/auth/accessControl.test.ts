import { describe, expect, it } from "vitest";
import type { CurrentUser } from "@/types";
import { canAccessAdminModules } from "./accessControl";

const user: CurrentUser = {
  id: "user-1",
  name: "测试用户",
  role: "employee",
  departments: [],
  permissions: [],
};

describe("accessControl", () => {
  it("allows only system administrators to access admin modules", () => {
    expect(canAccessAdminModules(undefined)).toBe(false);
    expect(canAccessAdminModules(user)).toBe(false);
    expect(canAccessAdminModules({ ...user, role: "manager" })).toBe(false);
    expect(canAccessAdminModules({ ...user, role: "admin" })).toBe(true);
  });
});
