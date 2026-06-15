import { mount } from "@vue/test-utils";
import { ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AppShell from "./AppShell.vue";

const mocks = vi.hoisted(() => ({
  push: vi.fn(),
  markMyNotificationRead: vi.fn(),
  refetchNotifications: vi.fn(),
}));

const currentUser = {
  id: "user-1",
  name: "员工",
  role: "employee" as const,
  departments: [],
  permissions: [],
};

const notifications = [
  {
    id: "notice-1",
    notification_type: "task_assigned",
    receiver_id: "user-1",
    task_id: "task-1",
    jump_url: "/tasks/task-1",
    content_summary: "新任务分配\n任务：需求确认",
    status: "success" as const,
    sent_at: "2026-06-10T08:00:00Z",
  },
  {
    id: "notice-2",
    notification_type: "task_overdue",
    receiver_id: "user-1",
    task_id: "task-2",
    jump_url: "/tasks/task-2",
    content_summary: "任务延期\n任务：接口联调",
    status: "success" as const,
    sent_at: "2026-06-09T08:00:00Z",
    read_at: "2026-06-09T09:00:00Z",
  },
];

vi.mock("vue-router", () => ({
  RouterLink: {
    props: ["to"],
    template: "<a><slot /></a>",
  },
  RouterView: {
    template: "<section />",
  },
  useRoute: () => ({ meta: { title: "工作台" } }),
  useRouter: () => ({ push: mocks.push }),
}));

vi.mock("@tanstack/vue-query", () => ({
  useQuery: vi.fn(({ queryKey }) => {
    if (queryKey[0] === "me") return { data: ref(currentUser) };
    if (queryKey[0] === "my-notifications") {
      return { data: ref(notifications), refetch: mocks.refetchNotifications };
    }
    return { data: ref([]) };
  }),
}));

vi.mock("@/api/client", () => ({
  api: {
    me: vi.fn(),
    myNotifications: vi.fn(),
    markMyNotificationRead: mocks.markMyNotificationRead,
  },
}));

vi.mock("@/assets/logo.png", () => ({
  default: "/logo.png",
}));

describe("AppShell notifications", () => {
  beforeEach(() => {
    mocks.push.mockClear();
    mocks.markMyNotificationRead.mockReset().mockResolvedValue(notifications[0]);
    mocks.refetchNotifications.mockClear();
  });

  it("shows current user notifications split by unread and read, then opens task detail", async () => {
    const wrapper = mount(AppShell, {
      global: {
        stubs: {
          ElIcon: { template: "<span><slot /></span>" },
          RouterLink: { template: "<a><slot /></a>" },
          RouterView: { template: "<section />" },
          UserAvatar: { template: "<span />" },
        },
      },
    });

    await wrapper.get(".notification-trigger").trigger("click");

    expect(wrapper.get(".notification-badge").text()).toBe("1");
    expect(wrapper.text()).toContain("未读");
    expect(wrapper.text()).toContain("新任务分配");
    expect(wrapper.text()).toContain("需求确认");

    await wrapper.get(".read-tab").trigger("click");

    expect(wrapper.text()).toContain("已读");
    expect(wrapper.text()).toContain("任务延期");
    expect(wrapper.text()).toContain("接口联调");

    await wrapper.get(".unread-tab").trigger("click");
    await wrapper.get("[data-notification-id='notice-1']").trigger("click");
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.markMyNotificationRead).toHaveBeenCalledWith("notice-1");
    expect(mocks.push).toHaveBeenCalledWith("/tasks/task-1");
  });

  it("does not render a chevron next to the current user name", () => {
    const wrapper = mount(AppShell, {
      global: {
        stubs: {
          ElIcon: { template: "<span><slot /></span>" },
          RouterLink: { template: "<a><slot /></a>" },
          RouterView: { template: "<section />" },
          UserAvatar: { template: "<span />" },
        },
      },
    });

    expect(wrapper.find(".chevron").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("⌄");
  });

  it("toggles the sidebar collapsed state from the collapse menu button", async () => {
    const wrapper = mount(AppShell, {
      global: {
        stubs: {
          ElIcon: { template: "<span><slot /></span>" },
          RouterLink: { template: "<a><slot /></a>" },
          RouterView: { template: "<section />" },
          UserAvatar: { template: "<span />" },
        },
      },
    });

    expect(wrapper.classes()).not.toContain("sidebar-collapsed");

    await wrapper.get(".collapse-button").trigger("click");

    expect(wrapper.classes()).toContain("sidebar-collapsed");
    expect(wrapper.get(".collapse-button").text()).toContain("展开菜单");
  });
});
