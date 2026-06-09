<script setup lang="ts">
import { computed } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { useRoute } from "vue-router";
import {
  Bell,
  Briefcase,
  Fold,
  Grid,
  QuestionFilled,
  Setting,
  User,
} from "@element-plus/icons-vue";
import { api } from "@/api/client";
import UserAvatar from "@/components/UserAvatar.vue";

const route = useRoute();
const { data: currentUser } = useQuery({
  queryKey: ["me"],
  queryFn: () => api.me(),
});

const pageTitle = computed(() => String(route.meta.title ?? "工作台"));

const navItems = [
  { to: "/workbench", label: "工作台", icon: Grid },
  { to: "/projects", label: "项目管理", icon: Briefcase },
  { to: "/users", label: "用户管理", icon: User },
  { to: "/settings", label: "系统设置", icon: Setting },
];
</script>

<template>
  <div class="app-shell">
    <aside class="app-sidebar">
      <RouterLink to="/workbench" class="brand">
        <span class="brand-mark">钉</span>
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
          <span v-if="pageTitle !== '项目管理'" class="dingtalk-hint">
            在钉钉桌面端使用
            <ElIcon><QuestionFilled /></ElIcon>
          </span>
        </div>
        <div class="header-actions">
          <button class="icon-button" type="button" aria-label="帮助">
            <ElIcon><QuestionFilled /></ElIcon>
          </button>
          <button class="icon-button with-badge" type="button" aria-label="通知">
            <ElIcon><Bell /></ElIcon>
            <span>12</span>
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
