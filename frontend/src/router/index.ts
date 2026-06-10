import { createRouter, createWebHistory } from "vue-router";
import type { RouteLocationNormalized } from "vue-router";
import AppShell from "@/layout/AppShell.vue";
import WorkbenchPage from "@/features/workbench/WorkbenchPage.vue";
import ProjectsPage from "@/features/projects/ProjectsPage.vue";
import UsersPage from "@/features/users/UsersPage.vue";
import SettingsPage from "@/features/settings/SettingsPage.vue";
import TaskDetailPage from "@/features/tasks/TaskDetailPage.vue";
import TaskFormPage from "@/features/tasks/TaskFormPage.vue";
import UnauthorizedPage from "@/features/auth/UnauthorizedPage.vue";
import { api } from "@/api/client";
import { canAccessAdminModules } from "@/auth/accessControl";
import { loadCurrentUserWithDingtalkLogin, resolveAdminToken } from "@/auth/dingtalkAuth";
import type { CurrentUser } from "@/types";

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
          meta: { title: "用户管理", requiresAdmin: true },
        },
        {
          path: "settings",
          name: "settings",
          component: SettingsPage,
          meta: { title: "系统设置", requiresAdmin: true },
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

let hasAuthenticated = false;
let authenticatedUser: CurrentUser | undefined;

function logSafePath(fullPath: string): string {
  return fullPath.replace(/([?&]admin-token=)[^&]*/u, "$1***");
}

function requiresAdmin(to: RouteLocationNormalized): boolean {
  return to.matched.some((record) => record.meta.requiresAdmin);
}

async function ensureAuthenticatedUser(): Promise<CurrentUser> {
  if (authenticatedUser) {
    return authenticatedUser;
  }
  authenticatedUser = await api.me({ skipUnauthorizedRedirect: true });
  return authenticatedUser;
}

async function guardAdminRoute(to: RouteLocationNormalized) {
  if (!requiresAdmin(to)) {
    return true;
  }

  const currentUser = await ensureAuthenticatedUser();
  if (canAccessAdminModules(currentUser)) {
    return true;
  }
  console.warn("[auth:router] admin route access denied", {
    targetPath: logSafePath(to.fullPath),
    role: currentUser.role,
  });
  return { name: "unauthorized" };
}

router.beforeEach(async (to) => {
  if (to.name === "unauthorized" || hasAuthenticated) {
    if (to.name === "unauthorized") {
      console.info("[auth:router] entering 401 page");
      return true;
    }
    return guardAdminRoute(to);
  }

  const adminTokenFromQuery = to.query["admin-token"];
  const adminToken =
    typeof adminTokenFromQuery === "string"
      ? adminTokenFromQuery
      : resolveAdminToken(
          to.fullPath.includes("?") ? to.fullPath.slice(to.fullPath.indexOf("?")) : "",
        );
  const targetPath = logSafePath(to.fullPath);
  console.info("[auth:router] auth guard started", {
    targetPath,
  });

  if (adminToken) {
    try {
      console.info("[auth:router] admin token login started", { targetPath });
      authenticatedUser = await api.adminTokenLogin(adminToken);
      hasAuthenticated = true;
      const query = { ...to.query };
      delete query["admin-token"];
      console.info("[auth:router] admin token login passed", { targetPath });
      const accessResult = await guardAdminRoute(to);
      if (accessResult !== true) {
        return accessResult;
      }
      return { path: to.path, query, hash: to.hash, replace: true };
    } catch {
      console.warn("[auth:router] admin token login failed", { targetPath });
      return { name: "unauthorized" };
    }
  }

  try {
    authenticatedUser = await loadCurrentUserWithDingtalkLogin({ authApi: api });
    hasAuthenticated = true;
    console.info("[auth:router] auth guard passed", {
      targetPath,
    });
    return guardAdminRoute(to);
  } catch {
    console.warn("[auth:router] auth guard redirected to 401", {
      targetPath,
    });
    return { name: "unauthorized" };
  }
});
