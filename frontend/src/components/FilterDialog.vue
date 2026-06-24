<script setup lang="ts">
import { reactive, watch } from "vue";
import type { Project, TaskFilters, User } from "@/types";

const props = defineProps<{
  modelValue: boolean;
  filters: TaskFilters;
  projects: Project[];
  users: User[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  apply: [filters: TaskFilters];
  reset: [];
}>();

const local = reactive<TaskFilters>({});

function toArray<T>(value?: T | T[]): T[] {
  if (Array.isArray(value)) return value;
  return value ? [value] : [];
}

watch(
  () => props.filters,
  (filters) => {
    Object.assign(local, {
      keyword: undefined,
      project_id: undefined,
      assignee_id: undefined,
      status: undefined,
      priority: undefined,
      date_from: undefined,
      date_to: undefined,
      ...filters,
      project_ids: toArray(filters.project_ids ?? filters.project_id),
      assignee_ids: toArray(filters.assignee_ids ?? filters.assignee_id),
      statuses: toArray(filters.statuses ?? filters.status),
      priorities: toArray(filters.priorities ?? filters.priority),
    });
  },
  { immediate: true },
);

function applyFilters() {
  const next: TaskFilters = {
    ...local,
    project_id: undefined,
    assignee_id: undefined,
    status: undefined,
    priority: undefined,
    project_ids: local.project_ids?.length ? local.project_ids : undefined,
    assignee_ids: local.assignee_ids?.length ? local.assignee_ids : undefined,
    statuses: local.statuses?.length ? local.statuses : undefined,
    priorities: local.priorities?.length ? local.priorities : undefined,
  };
  emit("apply", next);
  emit("update:modelValue", false);
}

function resetFilters() {
  Object.keys(local).forEach((key) => {
    delete local[key as keyof TaskFilters];
  });
  emit("reset");
  emit("update:modelValue", false);
}
</script>

<template>
  <ElDialog
    :model-value="modelValue"
    title="筛选条件"
    width="720px"
    class="filter-dialog"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <ElForm label-width="76px" class="filter-form">
      <ElFormItem label="项目">
        <ElSelect
          v-model="local.project_ids"
          multiple
          filterable
          clearable
          collapse-tags
          collapse-tags-tooltip
          placeholder="请选择项目"
        >
          <ElOption
            v-for="project in projects"
            :key="project.id"
            :label="project.name"
            :value="project.id"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="人员">
        <ElSelect
          v-model="local.assignee_ids"
          multiple
          filterable
          clearable
          collapse-tags
          collapse-tags-tooltip
          placeholder="请选择人员"
        >
          <ElOption
            v-for="user in users"
            :key="user.id"
            :label="user.name"
            :value="user.id"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect
          v-model="local.statuses"
          multiple
          clearable
          collapse-tags
          collapse-tags-tooltip
          placeholder="请选择状态"
        >
          <ElOption label="待开始" value="todo" />
          <ElOption label="进行中" value="in_progress" />
          <ElOption label="阻塞" value="blocked" />
          <ElOption label="已完成" value="done" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="优先级">
        <ElSelect
          v-model="local.priorities"
          multiple
          clearable
          collapse-tags
          collapse-tags-tooltip
          placeholder="请选择优先级"
        >
          <ElOption label="低" value="low" />
          <ElOption label="中" value="medium" />
          <ElOption label="高" value="high" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="日期范围">
        <div class="date-range">
          <ElDatePicker
            v-model="local.date_from"
            value-format="YYYY-MM-DD"
            placeholder="选择开始日期"
          />
          <span>~</span>
          <ElDatePicker
            v-model="local.date_to"
            value-format="YYYY-MM-DD"
            placeholder="选择结束日期"
          />
        </div>
      </ElFormItem>
      <ElFormItem label="关键词">
        <ElInput v-model="local.keyword" placeholder="请输入任务标题关键词" />
      </ElFormItem>
    </ElForm>
    <template #footer>
      <ElButton @click="resetFilters">重置</ElButton>
      <ElButton type="primary" @click="applyFilters">确定</ElButton>
    </template>
  </ElDialog>
</template>
