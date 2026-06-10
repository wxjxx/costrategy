<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
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
const currentPage = ref(1);
const pageSize = ref(10);
const pagedTasks = computed(() => pageRows(props.tasks, currentPage.value, pageSize.value));

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
    <ElTableColumn prop="title" label="任务标题" min-width="170" />
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
    <ElTableColumn label="操作" width="100">
      <template #default="{ row }">
        <ElButton link type="primary" @click.stop="router.push(`/tasks/${row.id}/edit`)">编辑</ElButton>
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
      :total="tasks.length"
    />
  </div>
</template>
