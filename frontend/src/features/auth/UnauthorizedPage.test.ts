import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import UnauthorizedPage from "./UnauthorizedPage.vue";

vi.mock("vue-router", () => ({
  useRouter: () => ({
    push: vi.fn(),
  }),
}));

describe("UnauthorizedPage", () => {
  it("renders the requested 401 no-permission copy and home action", () => {
    const wrapper = mount(UnauthorizedPage, {
      global: {
        stubs: {
          ElButton: {
            template: "<button><slot /></button>",
          },
          ElIcon: {
            template: "<span><slot /></span>",
          },
        },
      },
    });

    expect(wrapper.get("img").attributes("alt")).toBe("401 无权限访问插画");
    expect(wrapper.text()).toContain("抱歉，您无权访问此页面");
    expect(wrapper.text()).toContain("错误代码： 401 Unauthorized");
    expect(wrapper.text()).toContain("返回首页");
  });
});
