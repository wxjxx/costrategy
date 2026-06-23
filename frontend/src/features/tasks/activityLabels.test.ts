import { describe, expect, it } from "vitest";
import { activityActionLabel, activityStatusLabel } from "./activityLabels";

describe("activityActionLabel", () => {
  it("renders known activity codes in Chinese", () => {
    expect(activityActionLabel("task_created")).toBe("创建任务");
    expect(activityActionLabel("comment_created")).toBe("新增评论");
  });

  it("keeps unknown activity codes visible", () => {
    expect(activityActionLabel("custom_action")).toBe("custom_action");
  });

  it("renders known task statuses in Chinese", () => {
    expect(activityStatusLabel("blocked")).toBe("阻塞");
    expect(activityStatusLabel("done")).toBe("已完成");
  });
});
