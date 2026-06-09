import { createRouter, createWebHistory } from "vue-router";
import AppShell from "@/layout/AppShell.vue";
import WorkbenchPage from "@/features/workbench/WorkbenchPage.vue";
import ProjectsPage from "@/features/projects/ProjectsPage.vue";
import UsersPage from "@/features/users/UsersPage.vue";
import SettingsPage from "@/features/settings/SettingsPage.vue";
import TaskDetailPage from "@/features/tasks/TaskDetailPage.vue";
import TaskFormPage from "@/features/tasks/TaskFormPage.vue";
import UnauthorizedPage from "@/features/auth/UnauthorizedPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/401",
      name: "unauthorized",
      component: UnauthorizedPage,
      meta: { title: "未登录" },
    },
    {
      path: "/",
      component: AppShell,
      redirect: "/workbench",
      children: [
        {
          path: "workbench",
          name: "workbench",
          component: WorkbenchPage,
          meta: { title: "工作台" },
        },
        {
          path: "projects",
          name: "projects",
          component: ProjectsPage,
          meta: { title: "项目管理" },
        },
        {
          path: "users",
          name: "users",
          component: UsersPage,
          meta: { title: "用户管理" },
        },
        {
          path: "settings",
          name: "settings",
          component: SettingsPage,
          meta: { title: "系统设置" },
        },
        {
          path: "tasks/new",
          name: "task-new",
          component: TaskFormPage,
          meta: { title: "新建任务" },
        },
        {
          path: "tasks/:id",
          name: "task-detail",
          component: TaskDetailPage,
          meta: { title: "任务详情" },
        },
        {
          path: "tasks/:id/edit",
          name: "task-edit",
          component: TaskFormPage,
          meta: { title: "编辑任务" },
        },
      ],
    },
  ],
});
