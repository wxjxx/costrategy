import { describe, expect, it } from "vitest";
import source from "./TaskDetailPage.vue?raw";

describe("task detail layout", () => {
  it("sets an explicit max height on the activity scrollbar component", () => {
    expect(source).toMatch(
      /<ElScrollbar\s+class="activity-scrollbar"\s+max-height="220px">/,
    );
  });
});
