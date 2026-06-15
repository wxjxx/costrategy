<script setup lang="ts">
import { computed, ref } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import {
  Bell,
  Briefcase,
  Fold,
  Grid,
  Setting,
  User,
} from "@element-plus/icons-vue";
import { api } from "@/api/client";
import { canAccessAdminModules } from "@/auth/accessControl";
import UserAvatar from "@/components/UserAvatar.vue";
import logoUrl from "@/assets/logo.png";
import type { NotificationRecord } from "@/types";

const route = useRoute();
const router = useRouter();
const { data: currentUser } = useQuery({
  queryKey: ["me"],
  queryFn: () => api.me(),
});
const { data: myNotifications, refetch: refetchMyNotifications } = useQuery({
  queryKey: ["my-notifications", "header"],
  queryFn: api.myNotifications,
});

const sidebarCollapsed = ref(false);
const notificationPanelOpen = ref(false);
const activeNotificationTab = ref<"unread" | "read">("unread");
const pageTitle = computed(() => String(route.meta.title ?? "工作台"));
const unreadNotifications = computed(() =>
  (myNotifications.value ?? []).filter((record) => !record.read_at),
);
const readNotifications = computed(() =>
  (myNotifications.value ?? []).filter((record) => Boolean(record.read_at)),
);
const visibleNotifications = computed(() =>
  activeNotificationTab.value === "unread"
    ? unreadNotifications.value
    : readNotifications.value,
);
const notificationCount = computed(() => unreadNotifications.value.length);
const notificationBadgeText = computed(() =>
  notificationCount.value > 99 ? "99+" : String(notificationCount.value),
);

const navItems = computed(() =>
  [
    { to: "/workbench", label: "工作台", icon: Grid },
    { to: "/projects", label: "项目管理", icon: Briefcase },
    { to: "/users", label: "用户管理", icon: User, adminOnly: true },
    { to: "/settings", label: "系统设置", icon: Setting, adminOnly: true },
  ].filter((item) => !item.adminOnly || canAccessAdminModules(currentUser.value)),
);

function toggleNotificationPanel() {
  notificationPanelOpen.value = !notificationPanelOpen.value;
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}

function notificationTitle(record: NotificationRecord): string {
  return record.content_summary.split("\n")[0] || "通知消息";
}

function notificationDetail(record: NotificationRecord): string {
  return record.content_summary
    .split("\n")
    .slice(1)
    .join(" ")
    .replace(/^任务：/u, "")
    .trim();
}

function notificationTime(record: NotificationRecord): string {
  const matched = record.sent_at.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/u);
  return matched ? `${matched[1]} ${matched[2]}` : record.sent_at;
}

function notificationTarget(record: NotificationRecord): string {
  return record.jump_url ?? (record.task_id ? `/tasks/${record.task_id}` : "/workbench");
}

async function openNotification(record: NotificationRecord) {
  if (!record.read_at) {
    await api.markMyNotificationRead(record.id);
    await refetchMyNotifications();
  }
  notificationPanelOpen.value = false;
  void router.push(notificationTarget(record));
}
</script>

<template>
  <div class="app-shell" :class="{ 'sidebar-collapsed': sidebarCollapsed }">
    <aside class="app-sidebar">
      <RouterLink to="/workbench" class="brand">
        <img class="brand-mark" :src="logoUrl" alt="项目管理系统" />
        <span class="brand-text">项目管理系统</span>
      </RouterLink>
      <nav class="side-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="side-nav-item"
        >
          <ElIcon><component :is="item.icon" /></ElIcon>
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>
      <button
        class="collapse-button"
        type="button"
        :aria-label="sidebarCollapsed ? '展开菜单' : '收起菜单'"
        @click="toggleSidebar"
      >
        <ElIcon><Fold /></ElIcon>
        <span>{{ sidebarCollapsed ? "展开菜单" : "收起菜单" }}</span>
      </button>
    </aside>
    <section class="app-main">
      <header class="app-header">
        <div class="header-title">
          <h1>{{ pageTitle }}</h1>
        </div>
        <div class="header-actions">
          <button
            class="icon-button with-badge notification-trigger"
            type="button"
            :aria-label="`通知${notificationCount ? `（${notificationCount}）` : ''}`"
            @click="toggleNotificationPanel"
          >
            <ElIcon><Bell /></ElIcon>
            <span v-if="notificationCount > 0" class="notification-badge">
              {{ notificationBadgeText }}
            </span>
          </button>
          <section v-if="notificationPanelOpen" class="notification-panel">
            <div class="notification-tabs">
              <button
                class="unread-tab"
                :class="{ active: activeNotificationTab === 'unread' }"
                type="button"
                @click="activeNotificationTab = 'unread'"
              >
                未读 {{ unreadNotifications.length }}
              </button>
              <button
                class="read-tab"
                :class="{ active: activeNotificationTab === 'read' }"
                type="button"
                @click="activeNotificationTab = 'read'"
              >
                已读 {{ readNotifications.length }}
              </button>
            </div>
            <div v-if="visibleNotifications.length === 0" class="notification-empty">
              暂无通知
            </div>
            <button
              v-for="record in visibleNotifications"
              :key="record.id"
              class="notification-item"
              type="button"
              :data-notification-id="record.id"
              @click="openNotification(record)"
            >
              <span class="notification-item-title">{{ notificationTitle(record) }}</span>
              <span v-if="notificationDetail(record)" class="notification-item-detail">
                {{ notificationDetail(record) }}
              </span>
              <span class="notification-item-time">{{ notificationTime(record) }}</span>
            </button>
          </section>
          <UserAvatar :name="currentUser?.name" :size="38" />
          <strong>{{ currentUser?.name ?? "未登录" }}</strong>
        </div>
      </header>
      <main class="page-content">
        <RouterView />
      </main>
    </section>
  </div>
</template>
