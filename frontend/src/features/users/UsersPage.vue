<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { Refresh, Search } from "@element-plus/icons-vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { api } from "@/api/client";
import UserAvatar from "@/components/UserAvatar.vue";
import type { UserRole, UserStatus } from "@/types";
import { formatDateTimeInShanghai } from "@/utils/datetime";

const queryClient = useQueryClient();
const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.users });

const keyword = ref("");
const role = ref<UserRole | "">("");
const status = ref<UserStatus | "">("");
const currentPage = ref(1);
const pageSize = ref(10);
const roleDialog = ref(false);
const selectedUserId = ref("");
const selectedRole = ref<UserRole>("employee");
const selectedRows = ref<Array<{ id: string; name: string; status: UserStatus }>>([]);

const filteredUsers = computed(() =>
  (users.value ?? []).filter((user) => {
    if (keyword.value && !`${user.name}${user.mobile ?? ""}`.includes(keyword.value)) return false;
    if (role.value && user.role !== role.value) return false;
    if (status.value && user.status !== status.value) return false;
    return true;
  }),
);
const pagedUsers = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  return filteredUsers.value.slice(start, start + pageSize.value);
});

watch([keyword, role, status], () => {
  currentPage.value = 1;
});

watch(filteredUsers, (items) => {
  const maxPage = Math.max(1, Math.ceil(items.length / pageSize.value));
  if (currentPage.value > maxPage) currentPage.value = maxPage;
});
const latestSyncedAt = computed(() => {
  const timestamps = (users.value ?? [])
    .map((user) => user.last_synced_at)
    .filter((value): value is string => Boolean(value))
    .sort();
  return formatDateTimeInShanghai(timestamps.at(-1));
});
const selectedUser = computed(() =>
  users.value?.find((user) => user.id === selectedUserId.value),
);
const activeSelectedRows = computed(() =>
  selectedRows.value.filter((user) => user.status === "active"),
);

const roleMutation = useMutation({
  mutationFn: () => api.updateUserRole(selectedUserId.value, selectedRole.value),
  onSuccess: () => ElMessage.success("角色已更新"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
});
const statusMutation = useMutation({
  mutationFn: ({ userId, status }: { userId: string; status: UserStatus }) =>
    api.updateUserStatus(userId, status),
  onSuccess: () => ElMessage.success("用户状态已更新"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
});
const batchDisableMutation = useMutation({
  mutationFn: async () => {
    await Promise.all(
      activeSelectedRows.value.map((user) => api.updateUserStatus(user.id, "disabled")),
    );
  },
  onSuccess: () => {
    ElMessage.success("已批量停用用户");
    selectedRows.value = [];
  },
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
});
const syncMutation = useMutation({
  mutationFn: api.syncDingtalk,
  onSuccess: () => ElMessage.success("通讯录同步已触发"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
});

function roleLabel(value: UserRole): string {
  return { employee: "员工", manager: "管理人员", admin: "系统管理员" }[value];
}

function openRoleDialog(userId: string, userRole: UserRole) {
  selectedUserId.value = userId;
  selectedRole.value = userRole;
  roleDialog.value = true;
}

function resetUserFilters() {
  keyword.value = "";
  role.value = "";
  status.value = "";
  currentPage.value = 1;
}

function updateSelectedRows(rows: Array<{ id: string; name: string; status: UserStatus }>) {
  selectedRows.value = rows;
}

async function batchDisableUsers() {
  if (activeSelectedRows.value.length === 0) {
    ElMessage.warning("请选择正常状态的用户");
    return;
  }
  try {
    await ElMessageBox.confirm(
      `确认停用选中的 ${activeSelectedRows.value.length} 个用户？`,
      "批量停用",
      {
        confirmButtonText: "停用",
        cancelButtonText: "取消",
        type: "warning",
      },
    );
    batchDisableMutation.mutate();
  } catch {
    // User cancelled.
  }
}
</script>

<template>
  <div class="users-page">
    <section class="metric-row">
      <article class="metric-card"><span>◷</span><p>最近同步时间</p><strong>{{ latestSyncedAt }}</strong></article>
      <article class="metric-card"><span>♙</span><p>用户总数</p><strong>{{ users?.length ?? 0 }} 人</strong></article>
      <article class="metric-card"><span>✓</span><p>启用用户数</p><strong>{{ filteredUsers.filter((user) => user.status === 'active').length }} 人</strong></article>
    </section>

    <section class="content-card search-panel">
      <ElForm label-position="top">
        <ElRow :gutter="34">
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="关键词（姓名/手机号）">
              <ElInput v-model="keyword" placeholder="请输入姓名或手机号">
                <template #suffix><ElIcon><Search /></ElIcon></template>
              </ElInput>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="系统角色">
              <ElSelect v-model="role" clearable placeholder="请选择">
                <ElOption label="员工" value="employee" />
                <ElOption label="管理人员" value="manager" />
                <ElOption label="系统管理员" value="admin" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="用户状态">
              <ElSelect v-model="status" clearable placeholder="请选择">
                <ElOption label="正常" value="active" />
                <ElOption label="停用" value="disabled" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6" class="search-actions">
            <ElButton @click="resetUserFilters">重置</ElButton>
            <ElButton type="primary">搜索</ElButton>
          </ElCol>
        </ElRow>
      </ElForm>
    </section>

    <section class="content-card">
      <div class="section-heading">
        <h2>用户列表（共 {{ filteredUsers.length }} 人）</h2>
        <div class="section-actions">
          <ElButton
            type="danger"
            plain
            :disabled="activeSelectedRows.length === 0"
            :loading="batchDisableMutation.isPending.value"
            @click="batchDisableUsers"
          >
            批量停用
          </ElButton>
          <ElButton type="primary" :loading="syncMutation.isPending.value" @click="syncMutation.mutate()">
            <ElIcon><Refresh /></ElIcon>
            立即同步
          </ElButton>
        </div>
      </div>
      <ElTable :data="pagedUsers" @selection-change="updateSelectedRows">
        <ElTableColumn type="selection" width="48" />
        <ElTableColumn label="头像" width="88"><template #default="{ row }"><UserAvatar :name="row.name" :src="row.avatar_url" :size="42" /></template></ElTableColumn>
        <ElTableColumn prop="name" label="姓名" width="110" />
        <ElTableColumn prop="mobile" label="手机号" width="150" />
        <ElTableColumn label="部门" min-width="160"><template #default="{ row }">{{ row.departments?.join("、") }}</template></ElTableColumn>
        <ElTableColumn label="系统角色" width="150"><template #default="{ row }">{{ roleLabel(row.role) }}</template></ElTableColumn>
        <ElTableColumn label="用户状态" width="110">
          <template #default="{ row }">
            <span class="state-dot" :class="{ disabled: row.status === 'disabled' }" />
            {{ row.status === "active" ? "正常" : "停用" }}
          </template>
        </ElTableColumn>
        <ElTableColumn label="操作" width="160">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openRoleDialog(row.id, row.role)">设置角色</ElButton>
            <ElButton
              link
              :type="row.status === 'active' ? 'danger' : 'primary'"
              @click="statusMutation.mutate({ userId: row.id, status: row.status === 'active' ? 'disabled' : 'active' })"
            >
              {{ row.status === "active" ? "停用" : "启用" }}
            </ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
      <div class="table-footer">
        <ElPagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          background
          layout="total, sizes, prev, pager, next, jumper"
          :page-sizes="[10, 20, 30, 50]"
          :total="filteredUsers.length"
        />
      </div>
    </section>

    <ElDialog v-model="roleDialog" title="设置系统角色" width="430">
      <p class="dialog-subtitle">用户：{{ selectedUser?.name ?? "-" }}</p>
      <ElRadioGroup v-model="selectedRole" class="role-radio">
        <ElRadio value="employee">员工</ElRadio>
        <ElRadio value="manager">管理人员</ElRadio>
        <ElRadio value="admin">系统管理员</ElRadio>
      </ElRadioGroup>
      <template #footer>
        <ElButton @click="roleDialog = false">取消</ElButton>
        <ElButton type="primary" @click="roleMutation.mutate(); roleDialog = false">确认</ElButton>
      </template>
    </ElDialog>
  </div>
</template>
