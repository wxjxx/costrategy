import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import OverdueBadge from "./OverdueBadge.vue";

describe("OverdueBadge", () => {
  it("renders the compact overdue circle marker", () => {
    const wrapper = mount(OverdueBadge);

    expect(wrapper.text()).toBe("延");
    expect(wrapper.classes()).toContain("overdue-badge");
    expect(wrapper.attributes("aria-label")).toBe("已延期");
  });
});
