<script setup lang="ts">
import { useRouter } from "vue-router";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import type { Task } from "@/types";
import { getDisplayStatus } from "@/features/tasks/taskWorkflow";

defineProps<{ tasks: Task[] }>();

const router = useRouter();
</script>

<template>
  <ElTable :data="tasks" class="task-table" @row-click="(row: Task) => router.push(`/tasks/${row.id}`)">
    <ElTableColumn type="index" width="54" />
    <ElTableColumn prop="title" label="任务标题" min-width="170" />
    <ElTableColumn prop="project_name" label="所属项目" min-width="180" />
    <ElTableColumn label="负责人" width="130">
      <template #default="{ row }">
        <span class="table-user"><UserAvatar :name="row.assignee_name" :size="28" />{{ row.assignee_name }}</span>
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
    <span>共 {{ tasks.length }} 条</span>
    <ElPagination layout="prev, pager, next, sizes, jumper" :total="tasks.length" :page-size="10" />
  </div>
</template>
