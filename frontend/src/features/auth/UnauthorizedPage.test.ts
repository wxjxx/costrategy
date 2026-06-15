import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import UnauthorizedPage from "./UnauthorizedPage.vue";

const mocks = vi.hoisted(() => ({
  push: vi.fn(),
  resetAuthenticationState: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRouter: () => ({
    push: mocks.push,
  }),
}));

vi.mock("@/auth/sessionState", () => ({
  resetAuthenticationState: mocks.resetAuthenticationState,
}));

describe("UnauthorizedPage", () => {
  beforeEach(() => {
    mocks.push.mockClear();
    mocks.resetAuthenticationState.mockClear();
  });

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

  it("clears cached authentication before returning home so login runs again", async () => {
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

    await wrapper.get(".home-button").trigger("click");

    expect(mocks.resetAuthenticationState).toHaveBeenCalledOnce();
    expect(mocks.push).toHaveBeenCalledWith("/");
    expect(
      mocks.resetAuthenticationState.mock.invocationCallOrder[0],
    ).toBeLessThan(mocks.push.mock.invocationCallOrder[0]);
  });
});
