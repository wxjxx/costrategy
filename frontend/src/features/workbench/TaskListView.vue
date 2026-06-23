<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRouter } from "vue-router";
import { api } from "@/api/client";
import OverdueBadge from "@/components/OverdueBadge.vue";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import type { Task } from "@/types";
import {
  getDisplayStatus,
  primaryTaskAssigneeName,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";
import { clampPage, pageRows } from "@/utils/pagination";

const props = defineProps<{ tasks: Task[] }>();

const router = useRouter();
const queryClient = useQueryClient();
const currentPage = ref(1);
const pageSize = ref(10);
const sortedTasks = computed(() =>
  [...props.tasks].sort((left, right) => right.updated_at.localeCompare(left.updated_at)),
);
const pagedTasks = computed(() => pageRows(sortedTasks.value, currentPage.value, pageSize.value));

const deleteMutation = useMutation({
  mutationFn: (taskId: string) => api.deleteTask(taskId),
  onSuccess: () => ElMessage.success("任务已删除"),
  onError: () => ElMessage.error("只有任务创建人或管理人员可以删除任务"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["tasks"] }),
});

async function deleteTask(task: Task) {
  try {
    await ElMessageBox.confirm(`确认删除任务“${task.title}”？`, "删除任务", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    deleteMutation.mutate(task.id);
  } catch {
    // User cancelled.
  }
}

watch(
  () => props.tasks.length,
  (total) => {
    currentPage.value = clampPage(currentPage.value, total, pageSize.value);
  },
);

watch(pageSize, () => {
  currentPage.value = 1;
});
</script>

<template>
  <ElTable :data="pagedTasks" class="task-table" @row-click="(row: Task) => router.push(`/tasks/${row.id}`)">
    <ElTableColumn type="index" width="54" :index="(index: number) => (currentPage - 1) * pageSize + index + 1" />
    <ElTableColumn label="任务标题" min-width="170">
      <template #default="{ row }">
        <span class="task-title-cell">
          <OverdueBadge v-if="row.is_overdue" />
          {{ row.title }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn prop="project_name" label="所属项目" min-width="180" />
    <ElTableColumn label="负责人" width="130">
      <template #default="{ row }">
        <span class="table-user">
          <UserAvatar :name="primaryTaskAssigneeName(row)" :size="28" />
          {{ taskAssigneeNames(row) }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn label="状态" sortable width="126">
      <template #default="{ row }">
        <StatusTag :status="getDisplayStatus(row)" />
      </template>
    </ElTableColumn>
    <ElTableColumn label="优先级" sortable width="112">
      <template #default="{ row }">
        <PriorityTag :priority="row.priority" />
      </template>
    </ElTableColumn>
    <ElTableColumn prop="start_date" label="开始日期" width="136" />
    <ElTableColumn prop="due_date" label="截止日期" sortable width="136" />
    <ElTableColumn label="操作" width="140">
      <template #default="{ row }">
        <ElButton link type="primary" @click.stop="router.push(`/tasks/${row.id}/edit`)">编辑</ElButton>
        <ElButton link type="danger" @click.stop="deleteTask(row)">删除</ElButton>
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
      :total="sortedTasks.length"
    />
  </div>
</template>
