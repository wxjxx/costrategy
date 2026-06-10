<script setup lang="ts">
import { computed } from "vue";
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

const route = useRoute();
const router = useRouter();
const { data: currentUser } = useQuery({
  queryKey: ["me"],
  queryFn: () => api.me(),
});
const canViewNotificationRecords = computed(() =>
  canAccessAdminModules(currentUser.value) &&
  (currentUser.value?.permissions.includes("view_notification_records") ?? false),
);
const { data: notificationRecords } = useQuery({
  queryKey: ["notification-records", "header"],
  queryFn: api.notificationRecords,
  enabled: canViewNotificationRecords,
});

const pageTitle = computed(() => String(route.meta.title ?? "工作台"));
const notificationCount = computed(() => notificationRecords.value?.length ?? 0);
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

function openNotificationRecords() {
  void router.push({ path: "/settings", query: { tab: "records" } });
}
</script>

<template>
  <div class="app-shell">
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
      <button class="collapse-button" type="button">
        <ElIcon><Fold /></ElIcon>
        <span>收起菜单</span>
      </button>
    </aside>
    <section class="app-main">
      <header class="app-header">
        <div class="header-title">
          <h1>{{ pageTitle }}</h1>
        </div>
        <div class="header-actions">
          <button
            class="icon-button with-badge"
            type="button"
            :aria-label="`通知${notificationCount ? `（${notificationCount}）` : ''}`"
            @click="openNotificationRecords"
          >
            <ElIcon><Bell /></ElIcon>
            <span v-if="notificationCount > 0">{{ notificationBadgeText }}</span>
          </button>
          <UserAvatar :name="currentUser?.name" :size="38" />
          <strong>{{ currentUser?.name ?? "未登录" }}</strong>
          <span class="chevron">⌄</span>
        </div>
      </header>
      <main class="page-content">
        <RouterView />
      </main>
    </section>
  </div>
</template>
