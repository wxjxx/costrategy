<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { api } from "@/api/client";
import type { NotificationRule } from "@/types";
import { formatDateTimeInShanghai } from "@/utils/datetime";
import { isDebugModeEnabled, setDebugMode } from "@/utils/debugMode";
import { clampPage, pageRows } from "@/utils/pagination";

const queryClient = useQueryClient();
const route = useRoute();
const router = useRouter();
const settingTabs = ["dingtalk", "sync", "notification", "records", "storage", "debug"] as const;
type SettingTab = (typeof settingTabs)[number];
function normalizeTab(value: unknown): SettingTab {
  return typeof value === "string" && settingTabs.includes(value as SettingTab)
    ? (value as SettingTab)
    : "dingtalk";
}
const activeTab = ref<SettingTab>(normalizeTab(route.query.tab));
const settingDrafts = reactive<Record<string, string>>({});
const loadedSettingValues = reactive<Record<string, string>>({});
const { data: settings, error: settingsError } = useQuery({
  queryKey: ["settings"],
  queryFn: api.settings,
});
const { data: syncLogs } = useQuery({ queryKey: ["dingtalk-sync-logs"], queryFn: api.syncLogs });
const { data: rules } = useQuery({ queryKey: ["notification-rules"], queryFn: api.notificationRules });
const { data: records } = useQuery({ queryKey: ["notification-records"], queryFn: api.notificationRecords });
const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.users });
const syncLogPage = ref(1);
const syncLogPageSize = ref(10);
const recordPage = ref(1);
const recordPageSize = ref(10);
const debugModeEnabled = ref(isDebugModeEnabled());

const dingtalkSettings = computed(() =>
  (settings.value?.settings ?? []).filter((item) => item.group === "dingtalk"),
);
const rustfsSettings = computed(() =>
  (settings.value?.settings ?? []).filter((item) => item.group === "rustfs"),
);
const latestSyncLog = computed(() => syncLogs.value?.[0]);
const pagedSyncLogs = computed(() =>
  pageRows(syncLogs.value ?? [], syncLogPage.value, syncLogPageSize.value),
);
const pagedRecords = computed(() =>
  pageRows(records.value ?? [], recordPage.value, recordPageSize.value),
);
const syncMutation = useMutation({
  mutationFn: api.syncDingtalk,
  onSuccess: () => ElMessage.success("通讯录同步已触发"),
  onError: () => ElMessage.error("通讯录同步失败，请查看后端日志"),
  onSettled: () => {
    void queryClient.invalidateQueries({ queryKey: ["dingtalk-sync-logs"] });
  },
});
const ruleMutation = useMutation({
  mutationFn: ({
    ruleType,
    enabled,
  }: {
    ruleType: NotificationRule["rule_type"];
    enabled: boolean;
  }) => api.updateNotificationRule(ruleType, enabled),
  onSuccess: () => ElMessage.success("通知规则已更新"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["notification-rules"] }),
});
const settingsMutation = useMutation({
  mutationFn: () => {
    const updates = (settings.value?.settings ?? [])
      .map((item) => ({
        item,
        value: settingDrafts[item.key]?.trim() ?? "",
        loadedValue: loadedSettingValues[item.key] ?? "",
      }))
      .filter(({ item, value, loadedValue }) => {
        if (!value) return false;
        if (item.sensitive) return true;
        return value !== loadedValue;
      })
      .map(({ item, value }) => ({ key: item.key, value }));
    if (updates.length === 0) {
      throw new Error("no settings changed");
    }
    return api.updateSettings(updates);
  },
  onSuccess: () => {
    ElMessage.success("配置已保存");
    void queryClient.invalidateQueries({ queryKey: ["settings"] });
  },
  onError: (error) => {
    if (error instanceof Error && error.message === "no settings changed") {
      ElMessage.info("没有需要保存的配置");
      return;
    }
    ElMessage.error("配置保存失败，请查看后端日志");
  },
});

watch(
  () => settings.value?.settings,
  (items) => {
    for (const item of items ?? []) {
      const draftValue = item.sensitive ? "" : item.value_masked ?? "";
      settingDrafts[item.key] = draftValue;
      loadedSettingValues[item.key] = draftValue;
    }
  },
  { immediate: true },
);

watch(
  () => route.query.tab,
  (tab) => {
    activeTab.value = normalizeTab(tab);
  },
);

watch(activeTab, (tab) => {
  if (route.query.tab === tab) return;
  void router.replace({ query: { ...route.query, tab } });
});

watch(syncLogs, (items) => {
  syncLogPage.value = clampPage(syncLogPage.value, items?.length ?? 0, syncLogPageSize.value);
});

watch(syncLogPageSize, () => {
  syncLogPage.value = 1;
});

watch(records, (items) => {
  recordPage.value = clampPage(recordPage.value, items?.length ?? 0, recordPageSize.value);
});

watch(recordPageSize, () => {
  recordPage.value = 1;
});

function ruleLabel(ruleType: string): string {
  return {
    task_assigned: "新任务分配通知负责人",
    assignee_changed: "负责人变更通知新负责人",
    task_commented: "新评论通知负责人",
    due_tomorrow: "截止前一天提醒负责人",
    task_overdue: "任务延期通知负责人和项目负责人",
  }[ruleType] ?? ruleType;
}

function notificationTypeLabel(notificationType: string): string {
  return {
    task_assigned: "新任务分配",
    assignee_changed: "负责人变更",
    task_commented: "新评论",
    due_tomorrow: "截止前一天提醒",
    task_overdue: "任务延期",
  }[notificationType] ?? notificationType;
}

function receiverName(receiverId: string): string {
  return users.value?.find((user) => user.id === receiverId)?.name ?? receiverId;
}

function formatDateTime(value?: string): string {
  return formatDateTimeInShanghai(value);
}

function notificationTaskId(row: { task_id?: string; content_summary: string }): string | undefined {
  return row.task_id ?? row.content_summary.match(/task_id=([0-9a-f-]{36})/iu)?.[1];
}

function notificationJumpUrl(row: { jump_url?: string; task_id?: string; content_summary: string }): string | undefined {
  return row.jump_url ?? (notificationTaskId(row) ? `/tasks/${notificationTaskId(row)}` : undefined);
}

function notificationSummaryText(content: string): string {
  return content.replace(/\n?进入任务详情：.*$/su, "").trim();
}

function absoluteUrl(path: string): string {
  return path.startsWith("http") ? path : `${window.location.origin}${path}`;
}

function syncStatusLabel(status?: string): string {
  return { running: "运行中", success: "成功", failed: "失败" }[status ?? ""] ?? "-";
}

function settingSourceLabel(source: string): string {
  return { database: "数据库", env: "环境变量", empty: "未配置" }[source] ?? source;
}

function settingDisplayValue(value?: string): string {
  return value && value.trim() ? value : "未配置";
}

function showConnectionStatus() {
  const status = settings.value?.connection_status;
  ElMessage.info(
    `钉钉：${status?.dingtalk ?? "-"}，RustFS：${status?.rustfs ?? "-"}`,
  );
}

function updateDebugMode(enabled: boolean) {
  debugModeEnabled.value = enabled;
  setDebugMode(enabled);
  ElMessage.success(enabled ? "调试模式已开启" : "调试模式已关闭");
}
</script>

<template>
  <div class="settings-page">
    <ElTabs v-model="activeTab" class="settings-tabs">
      <ElTabPane label="钉钉应用" name="dingtalk" />
      <ElTabPane label="通讯录同步" name="sync" />
      <ElTabPane label="通知配置" name="notification" />
      <ElTabPane label="通知记录" name="records" />
      <ElTabPane label="存储配置" name="storage" />
      <ElTabPane label="调试模式" name="debug" />
    </ElTabs>

    <section v-if="activeTab === 'dingtalk'" class="content-card">
      <h2>钉钉应用配置</h2>
      <ElAlert
        v-if="settingsError"
        title="钉钉应用配置加载失败，请确认当前账号有系统设置权限。"
        type="error"
        show-icon
        :closable="false"
      />
      <ElForm label-position="top" class="settings-form">
        <ElRow :gutter="40">
          <ElCol v-for="item in dingtalkSettings" :key="item.key" :xs="24" :sm="12" :md="8" :lg="6">
            <ElFormItem :label="item.label">
              <ElInput
                v-model="settingDrafts[item.key]"
                :type="item.sensitive ? 'password' : 'text'"
                :class="{ 'is-empty-setting': !item.configured }"
                :placeholder="settingDisplayValue(item.value_masked)"
                show-password
              />
              <ElTag class="setting-source" size="small" :type="item.configured ? 'success' : 'info'">
                {{ settingSourceLabel(item.source) }}
              </ElTag>
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <div class="page-actions inline">
        <ElButton @click="showConnectionStatus">测试连接</ElButton>
        <ElButton type="primary" :loading="settingsMutation.isPending.value" @click="settingsMutation.mutate()">保存配置</ElButton>
      </div>
    </section>

    <section v-if="activeTab === 'sync'" class="content-card">
      <h2>通讯录同步</h2>
      <div class="sync-controls">
        <ElButton type="primary" plain :loading="syncMutation.isPending.value" @click="syncMutation.mutate()">立即同步</ElButton>
        <ElSwitch model-value />
        <span>最近同步状态：{{ syncStatusLabel(latestSyncLog?.status) }}</span>
      </div>
      <ElTable :data="pagedSyncLogs" empty-text="暂无同步日志">
        <ElTableColumn label="同步状态">
          <template #default="{ row }">{{ syncStatusLabel(row.status) }}</template>
        </ElTableColumn>
        <ElTableColumn prop="created_users" label="新增用户数" />
        <ElTableColumn prop="updated_users" label="更新用户数" />
        <ElTableColumn prop="disabled_users" label="停用用户数" />
        <ElTableColumn prop="failure_reason" label="失败原因" />
      </ElTable>
      <div class="table-footer">
        <ElPagination
          v-model:current-page="syncLogPage"
          v-model:page-size="syncLogPageSize"
          background
          layout="total, sizes, prev, pager, next, jumper"
          :page-sizes="[10, 20, 30, 50]"
          :total="syncLogs?.length ?? 0"
        />
      </div>
    </section>

    <section v-if="activeTab === 'notification'" class="content-card">
      <h2>通知规则</h2>
      <div v-for="rule in rules ?? []" :key="rule.rule_type" class="rule-row">
        <ElSwitch
          :model-value="rule.enabled"
          @change="ruleMutation.mutate({ ruleType: rule.rule_type, enabled: Boolean($event) })"
        />
        <strong>{{ ruleLabel(rule.rule_type) }}</strong>
        <span>当规则触发时，向对应负责人发送钉钉个人通知。</span>
      </div>
      <ElAlert title="当前通知通过钉钉个人工作通知发送。" type="info" show-icon :closable="false" />
    </section>

    <section v-if="activeTab === 'records'" class="content-card">
      <h2>通知发送记录</h2>
      <ElTable :data="pagedRecords">
        <ElTableColumn label="通知类型" width="150">
          <template #default="{ row }">{{ notificationTypeLabel(row.notification_type) }}</template>
        </ElTableColumn>
        <ElTableColumn label="接收人" width="140">
          <template #default="{ row }">{{ receiverName(row.receiver_id) }}</template>
        </ElTableColumn>
        <ElTableColumn label="关联任务" min-width="360">
          <template #default="{ row }">
            <div class="notification-summary">
              <span>{{ notificationSummaryText(row.content_summary) }}</span>
              <template v-if="notificationJumpUrl(row)">
                <span>进入任务详情：</span>
                <a :href="absoluteUrl(notificationJumpUrl(row)!)" target="_blank" rel="noopener">
                  {{ absoluteUrl(notificationJumpUrl(row)!) }}
                </a>
              </template>
            </div>
          </template>
        </ElTableColumn>
        <ElTableColumn label="发送时间" width="180">
          <template #default="{ row }">{{ formatDateTime(row.sent_at) }}</template>
        </ElTableColumn>
        <ElTableColumn label="发送状态">
          <template #default="{ row }"><ElTag :type="row.status === 'success' ? 'success' : 'danger'">{{ row.status === "success" ? "成功" : "失败" }}</ElTag></template>
        </ElTableColumn>
        <ElTableColumn prop="failure_reason" label="失败原因" />
      </ElTable>
      <div class="table-footer">
        <ElPagination
          v-model:current-page="recordPage"
          v-model:page-size="recordPageSize"
          background
          layout="total, sizes, prev, pager, next, jumper"
          :page-sizes="[10, 20, 30, 50]"
          :total="records?.length ?? 0"
        />
      </div>
    </section>

    <section v-if="activeTab === 'storage'" class="content-card">
      <h2>RustFS 存储配置</h2>
      <ElForm label-position="top" class="settings-form">
        <ElRow :gutter="30">
          <ElCol v-for="item in rustfsSettings" :key="item.key" :xs="24" :sm="12" :md="8">
            <ElFormItem :label="item.label">
              <ElInput
                v-model="settingDrafts[item.key]"
                :type="item.sensitive ? 'password' : 'text'"
                :class="{ 'is-empty-setting': !item.configured }"
                :placeholder="settingDisplayValue(item.value_masked)"
                show-password
              />
              <ElTag class="setting-source" size="small" :type="item.configured ? 'success' : 'info'">
                {{ settingSourceLabel(item.source) }}
              </ElTag>
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <ElButton
        type="primary"
        class="full-width-button"
        :loading="settingsMutation.isPending.value"
        @click="settingsMutation.mutate()"
      >
        保存配置
      </ElButton>
    </section>

    <section v-if="activeTab === 'debug'" class="content-card">
      <h2>调试模式</h2>
      <div class="rule-row debug-rule-row">
        <ElSwitch
          :model-value="debugModeEnabled"
          @change="updateDebugMode(Boolean($event))"
        />
        <strong>启用 vConsole</strong>
        <span>开启后会在当前浏览器本地写入 debug=1，并显示移动端调试控制台。</span>
      </div>
    </section>
  </div>
</template>
