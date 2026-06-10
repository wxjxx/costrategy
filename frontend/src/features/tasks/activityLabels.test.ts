import { describe, expect, it } from "vitest";
import { activityActionLabel } from "./activityLabels";

describe("activityActionLabel", () => {
  it("renders known activity codes in Chinese", () => {
    expect(activityActionLabel("task_created")).toBe("创建任务");
    expect(activityActionLabel("comment_created")).toBe("新增评论");
  });

  it("keeps unknown activity codes visible", () => {
    expect(activityActionLabel("custom_action")).toBe("custom_action");
  });
});
