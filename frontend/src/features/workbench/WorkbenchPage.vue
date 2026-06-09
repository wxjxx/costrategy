<script setup lang="ts">
import { computed, ref } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { Filter, Plus } from "@element-plus/icons-vue";
import { useRouter } from "vue-router";
import { api } from "@/api/client";
import FilterDialog from "@/components/FilterDialog.vue";
import { useWorkbenchStore, type WorkbenchView } from "@/stores/workbench";
import TaskGanttView from "./TaskGanttView.vue";
import TaskKanbanView from "./TaskKanbanView.vue";
import TaskListView from "./TaskListView.vue";
import { filterTasks, priorityLabel, statusLabel } from "@/features/tasks/taskWorkflow";
import type { TaskFilters } from "@/types";

const router = useRouter();
const store = useWorkbenchStore();
const showFilters = ref(false);

const { data: currentUser } = useQuery({ queryKey: ["me"], queryFn: api.me });
const { data: tasksData } = useQuery({
  queryKey: ["tasks", computed(() => store.filters)],
  queryFn: () => api.tasks(store.filters),
});
const { data: projectsData } = useQuery({
  queryKey: ["projects"],
  queryFn: api.projects,
});
const { data: usersData } = useQuery({ queryKey: ["users"], queryFn: api.users });

const tasks = computed(() => filterTasks(tasksData.value ?? [], store.filters));
const projects = computed(() => projectsData.value ?? []);
const users = computed(() => usersData.value ?? []);

const viewTabs: Array<{ key: WorkbenchView; label: string }> = [
  { key: "kanban", label: "看板" },
  { key: "gantt", label: "甘特图" },
  { key: "list", label: "列表" },
];

const filterChips = computed(() => {
  const filters = store.filters;
  const chips: Array<{ key: keyof TaskFilters; label: string }> = [];
  const project = projects.value.find((item) => item.id === filters.project_id);
  const user = users.value.find((item) => item.id === filters.assignee_id);
  if (project) chips.push({ key: "project_id", label: `项目：${project.name}` });
  if (filters.status) chips.push({ key: "status", label: `状态：${statusLabel(filters.status)}` });
  if (filters.priority) chips.push({ key: "priority", label: `优先级：${priorityLabel(filters.priority)}` });
  if (user) chips.push({ key: "assignee_id", label: `负责人：${user.name}` });
  if (filters.keyword) chips.push({ key: "keyword", label: `关键词：${filters.keyword}` });
  return chips;
});

function applyFilters(filters: TaskFilters) {
  store.setFilters(filters);
}
</script>

<template>
  <div class="workbench-page">
    <section class="filter-strip">
      <ElButton type="primary" plain class="filter-button" @click="showFilters = true">
        <ElIcon><Filter /></ElIcon>
        筛选
      </ElButton>
      <ElTag
        v-for="chip in filterChips"
        :key="chip.key"
        closable
        effect="plain"
        @close="store.clearFilter(chip.key)"
      >
        {{ chip.label }}
      </ElTag>
    </section>

    <section class="content-card workbench-card">
      <div class="workbench-tabs">
        <button
          v-for="tab in viewTabs"
          :key="tab.key"
          type="button"
          :class="{ active: store.view === tab.key }"
          @click="store.setView(tab.key)"
        >
          {{ tab.label }}
        </button>
        <ElButton type="primary" class="create-task" @click="router.push('/tasks/new')">
          <ElIcon><Plus /></ElIcon>
          新建任务
        </ElButton>
      </div>
      <ElAlert
        v-if="store.view !== 'gantt'"
        class="view-alert"
        type="primary"
        show-icon
        :closable="false"
        title="员工仅可拖拽自己负责的任务，管理人员可拖拽全部任务"
      />
      <TaskKanbanView
        v-if="store.view === 'kanban' && currentUser"
        :tasks="tasks"
        :current-user="currentUser"
      />
      <TaskGanttView v-else-if="store.view === 'gantt'" :tasks="tasks" />
      <TaskListView v-else :tasks="tasks" />
    </section>

    <FilterDialog
      v-model="showFilters"
      :filters="store.filters"
      :projects="projects"
      :users="users"
      @apply="applyFilters"
      @reset="store.resetFilters"
    />
  </div>
</template>
