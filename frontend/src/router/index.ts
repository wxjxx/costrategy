import { createRouter, createWebHistory } from 'vue-router';
import AppLayout from '../layout/AppLayout.vue';

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/workbench',
    },
    {
      path: '/',
      component: AppLayout,
      children: [
        {
          path: 'workbench',
          component: () => import('../views/WorkbenchPage.vue'),
        },
        {
          path: 'projects',
          component: () => import('../views/ProjectsPage.vue'),
        },
        {
          path: 'users',
          component: () => import('../views/UsersPage.vue'),
        },
        {
          path: 'settings',
          component: () => import('../views/SettingsPage.vue'),
        },
      ],
    },
  ],
});
