import { describe, expect, it } from "vitest";
import type { User } from "@/types";
import { selectableUsers } from "./userOptions";

const activeUser: User = {
  id: "user-1",
  dingtalk_user_id: "ding-user-1",
  name: "张三",
  role: "employee",
  status: "active",
};

const disabledUser: User = {
  id: "user-2",
  dingtalk_user_id: "ding-user-2",
  name: "李四",
  role: "employee",
  status: "disabled",
};

describe("userOptions", () => {
  it("only exposes active users for project and task selectors", () => {
    expect(selectableUsers([activeUser, disabledUser])).toEqual([activeUser]);
  });
});
