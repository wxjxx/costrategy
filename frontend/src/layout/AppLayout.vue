<script setup lang="ts">
import {
  Bell,
  FolderKanban,
  LayoutDashboard,
  Settings,
  Users,
} from 'lucide-vue-next';
import { computed } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { getVisibleNavItems, type NavItem } from './menu';
import type { CurrentUser } from '../types/auth';

const props = withDefaults(
  defineProps<{
    currentUser?: CurrentUser;
  }>(),
  {
    currentUser: () => ({
      id: 'guest',
      name: '未登录',
      role: 'employee',
      departments: [],
      permissions: [],
    }),
  },
);

const route = useRoute();
const navItems = computed(() => getVisibleNavItems(props.currentUser.role));
const currentTitle = computed(() => {
  const current = navItems.value.find((item) => route.path.startsWith(item.path));
  return current?.label ?? '工作台';
});

const iconMap = {
  LayoutDashboard,
  FolderKanban,
  Users,
  Settings,
} as const;

function iconFor(item: NavItem) {
  return iconMap[item.icon];
}
</script>

<template>
  <div class="app-shell">
    <aside class="app-sidebar" aria-label="主导航">
      <div class="app-brand">
        <div class="app-brand__mark">项</div>
        <div>
          <strong>项目管理系统</strong>
          <span>企业内部 H5</span>
        </div>
      </div>

      <nav class="app-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.key"
          :to="item.path"
          class="app-nav__item"
          :class="{ 'app-nav__item--active': route.path.startsWith(item.path) }"
        >
          <component :is="iconFor(item)" :size="17" stroke-width="1.9" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>
    </aside>

    <section class="app-main">
      <header class="app-topbar">
        <div>
          <h1>{{ currentTitle }}</h1>
          <p>任务、排期和项目协作</p>
        </div>
        <div class="app-topbar__actions">
          <button class="icon-button" type="button" aria-label="通知">
            <Bell :size="17" stroke-width="1.9" />
          </button>
          <div class="user-chip">
            <span class="user-chip__avatar">{{ currentUser.name.slice(0, 1) }}</span>
            <span>{{ currentUser.name }}</span>
          </div>
        </div>
      </header>

      <main class="app-content">
        <RouterView />
      </main>
    </section>
  </div>
</template>
