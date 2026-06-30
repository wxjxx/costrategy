<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ElMessage } from "element-plus";
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
import { formatDateTimeInShanghai } from "@/utils/datetime";

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
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
const profileDialogOpen = ref(false);
const profileAvatarUrl = ref("");
const avatarFileInput = ref<HTMLInputElement>();
const notificationMenu = ref<HTMLElement>();
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
const saveAvatarMutation = useMutation({
  mutationFn: (avatarUrl: string) => api.updateMyAvatar(avatarUrl),
  onSuccess: (updatedUser) => {
    queryClient.setQueryData(["me"], updatedUser);
    void queryClient.invalidateQueries({ queryKey: ["users"] });
    void queryClient.invalidateQueries({ queryKey: ["projects"] });
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
    profileDialogOpen.value = false;
    ElMessage.success("头像已更新");
  },
});
const avatarSaving = computed(() => saveAvatarMutation.isPending.value);

const navItems = computed(() =>
  [
    { to: "/workbench", label: "工作台", icon: Grid },
    { to: "/projects", label: "项目管理", icon: Briefcase },
    { to: "/users", label: "用户管理", icon: User, adminOnly: true },
    { to: "/settings", label: "系统设置", icon: Setting, adminOnly: true },
  ].filter((item) => !item.adminOnly || canAccessAdminModules(currentUser.value)),
);

watch(
  () => currentUser.value?.avatar_url,
  (avatarUrl) => {
    if (!profileDialogOpen.value) {
      profileAvatarUrl.value = avatarUrl ?? "";
    }
  },
  { immediate: true },
);

onMounted(() => {
  document.addEventListener("click", closeNotificationPanelOnOutsideClick);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", closeNotificationPanelOnOutsideClick);
});

function toggleNotificationPanel() {
  notificationPanelOpen.value = !notificationPanelOpen.value;
}

function closeNotificationPanelOnOutsideClick(event: MouseEvent) {
  if (!notificationPanelOpen.value) return;
  const target = event.target;
  if (target instanceof Node && notificationMenu.value?.contains(target)) return;
  notificationPanelOpen.value = false;
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}

function openProfileDialog() {
  profileAvatarUrl.value = currentUser.value?.avatar_url ?? "";
  profileDialogOpen.value = true;
}

function clearProfileAvatar() {
  profileAvatarUrl.value = "";
}

function selectAvatarFile() {
  avatarFileInput.value?.click();
}

function handleAvatarFileChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  input.value = "";
  if (!file.type.startsWith("image/")) {
    ElMessage.warning("请选择图片文件");
    return;
  }
  if (file.size > 512 * 1024) {
    ElMessage.warning("头像图片不能超过 512KB");
    return;
  }

  const reader = new FileReader();
  reader.onload = () => {
    if (typeof reader.result === "string") {
      profileAvatarUrl.value = reader.result;
    }
  };
  reader.onerror = () => ElMessage.error("头像读取失败");
  reader.readAsDataURL(file);
}

async function saveProfileAvatar() {
  await saveAvatarMutation.mutateAsync(profileAvatarUrl.value);
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
  return formatDateTimeInShanghai(record.sent_at, false);
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
          <div ref="notificationMenu" class="notification-menu">
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
              <ElScrollbar class="notification-list-scrollbar">
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
              </ElScrollbar>
            </section>
          </div>
          <button
            class="profile-trigger"
            type="button"
            :aria-label="`${currentUser?.name ?? '用户'}头像管理`"
            @click="openProfileDialog"
          >
            <UserAvatar
              :name="currentUser?.name"
              :src="currentUser?.avatar_url"
              :size="38"
            />
            <strong>{{ currentUser?.name ?? "未登录" }}</strong>
          </button>
        </div>
      </header>
      <ElScrollbar tag="main" class="page-content page-content-scrollbar">
        <RouterView />
      </ElScrollbar>
    </section>
    <ElDialog
      v-model="profileDialogOpen"
      title="个人头像管理"
      width="420px"
      class="profile-dialog"
    >
      <div class="profile-avatar-editor">
        <UserAvatar
          :name="currentUser?.name"
          :src="profileAvatarUrl"
          :size="78"
        />
        <div class="profile-avatar-actions">
          <ElButton type="primary" plain @click="selectAvatarFile">
            上传头像
          </ElButton>
          <ElButton plain @click="clearProfileAvatar">
            移除头像
          </ElButton>
        </div>
        <input
          ref="avatarFileInput"
          class="hidden-file-input"
          type="file"
          accept="image/*"
          @change="handleAvatarFileChange"
        />
        <label class="avatar-url-field">
          <span>头像地址</span>
          <input
            v-model="profileAvatarUrl"
            class="avatar-url-input"
            type="url"
            placeholder="https:// 或 data:image/"
            @keyup.enter="saveProfileAvatar"
          />
        </label>
      </div>
      <template #footer>
        <ElButton @click="profileDialogOpen = false">取消</ElButton>
        <ElButton
          type="primary"
          class="profile-avatar-save"
          :loading="avatarSaving"
          @click="saveProfileAvatar"
        >
          保存
        </ElButton>
      </template>
    </ElDialog>
  </div>
</template>
