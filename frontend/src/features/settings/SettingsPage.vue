<script setup lang="ts">
import { computed, ref } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ElMessage } from "element-plus";
import { api } from "@/api/client";
import type { NotificationRule } from "@/types";

const queryClient = useQueryClient();
const activeTab = ref("dingtalk");
const { data: settings, error: settingsError } = useQuery({
  queryKey: ["settings"],
  queryFn: api.settings,
});
const { data: syncLogs } = useQuery({ queryKey: ["dingtalk-sync-logs"], queryFn: api.syncLogs });
const { data: rules } = useQuery({ queryKey: ["notification-rules"], queryFn: api.notificationRules });
const { data: records } = useQuery({ queryKey: ["notification-records"], queryFn: api.notificationRecords });

const dingtalkSettings = computed(() =>
  (settings.value?.settings ?? []).filter((item) => item.group === "dingtalk"),
);
const rustfsSettings = computed(() =>
  (settings.value?.settings ?? []).filter((item) => item.group === "rustfs"),
);
const latestSyncLog = computed(() => syncLogs.value?.[0]);
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

function ruleLabel(ruleType: string): string {
  return {
    task_assigned: "新任务分配通知负责人",
    assignee_changed: "负责人变更通知新负责人",
    due_tomorrow: "截止前一天提醒负责人",
    task_overdue: "任务延期通知负责人和项目负责人",
  }[ruleType] ?? ruleType;
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
</script>

<template>
  <div class="settings-page">
    <ElTabs v-model="activeTab" class="settings-tabs">
      <ElTabPane label="钉钉应用" name="dingtalk" />
      <ElTabPane label="通讯录同步" name="sync" />
      <ElTabPane label="通知配置" name="notification" />
      <ElTabPane label="通知记录" name="records" />
      <ElTabPane label="存储配置" name="storage" />
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
          <ElCol v-for="item in dingtalkSettings" :key="item.key" :span="6">
            <ElFormItem :label="item.label">
              <ElInput
                :model-value="settingDisplayValue(item.value_masked)"
                :type="item.sensitive ? 'password' : 'text'"
                :class="{ 'is-empty-setting': !item.configured }"
                show-password
                readonly
              />
              <ElTag class="setting-source" size="small" :type="item.configured ? 'success' : 'info'">
                {{ settingSourceLabel(item.source) }}
              </ElTag>
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <div class="page-actions inline">
        <ElButton>测试连接</ElButton>
        <ElButton type="primary">保存配置</ElButton>
      </div>
    </section>

    <section v-if="activeTab === 'sync'" class="content-card">
      <h2>通讯录同步</h2>
      <div class="sync-controls">
        <span>同步范围：</span>
        <ElCheckbox model-value>部门</ElCheckbox>
        <ElCheckbox model-value>用户</ElCheckbox>
        <ElCheckbox model-value>用户部门关系</ElCheckbox>
        <ElButton type="primary" plain :loading="syncMutation.isPending.value" @click="syncMutation.mutate()">立即同步</ElButton>
        <ElSwitch model-value />
        <span>最近同步状态：{{ syncStatusLabel(latestSyncLog?.status) }}</span>
      </div>
      <ElTable :data="syncLogs ?? []" empty-text="暂无同步日志">
        <ElTableColumn label="同步状态">
          <template #default="{ row }">{{ syncStatusLabel(row.status) }}</template>
        </ElTableColumn>
        <ElTableColumn prop="created_users" label="新增用户数" />
        <ElTableColumn prop="updated_users" label="更新用户数" />
        <ElTableColumn prop="disabled_users" label="停用用户数" />
        <ElTableColumn prop="failure_reason" label="失败原因" />
      </ElTable>
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
      <ElAlert
        title="第一版不支持：新评论通知、新附件通知、钉钉群机器人通知、钉钉待办、钉钉审批。"
        type="warning"
        show-icon
        :closable="false"
      />
    </section>

    <section v-if="activeTab === 'records'" class="content-card">
      <h2>通知发送记录</h2>
      <ElTable :data="records ?? []">
        <ElTableColumn prop="notification_type" label="通知类型" />
        <ElTableColumn prop="receiver_id" label="接收人" />
        <ElTableColumn prop="content_summary" label="关联任务" />
        <ElTableColumn prop="sent_at" label="发送时间" />
        <ElTableColumn label="发送状态">
          <template #default="{ row }"><ElTag :type="row.status === 'success' ? 'success' : 'danger'">{{ row.status === "success" ? "成功" : "失败" }}</ElTag></template>
        </ElTableColumn>
        <ElTableColumn prop="failure_reason" label="失败原因" />
      </ElTable>
    </section>

    <section v-if="activeTab === 'storage'" class="content-card">
      <h2>RustFS 存储配置</h2>
      <ElForm label-position="top" class="settings-form">
        <ElRow :gutter="30">
          <ElCol v-for="item in rustfsSettings" :key="item.key" :span="8">
            <ElFormItem :label="item.label">
              <ElInput
                :model-value="settingDisplayValue(item.value_masked)"
                :type="item.sensitive ? 'password' : 'text'"
                :class="{ 'is-empty-setting': !item.configured }"
                show-password
                readonly
              />
              <ElTag class="setting-source" size="small" :type="item.configured ? 'success' : 'info'">
                {{ settingSourceLabel(item.source) }}
              </ElTag>
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <ElButton type="primary" class="full-width-button">保存配置</ElButton>
    </section>
  </div>
</template>
