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

const { data: currentUser } = useQuery({
  queryKey: ["me"],
  queryFn: () => api.me(),
});
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
const isMyTasksFilterActive = computed(
  () =>
    Boolean(currentUser.value) &&
    selectedValues(store.filters.assignee_ids, store.filters.assignee_id).includes(
      currentUser.value?.id ?? "",
    ),
);

const viewTabs: Array<{ key: WorkbenchView; label: string }> = [
  { key: "kanban", label: "看板" },
  { key: "gantt", label: "甘特图" },
  { key: "list", label: "列表" },
];

const filterChips = computed(() => {
  const filters = store.filters;
  const chips: Array<{ key: keyof TaskFilters; label: string }> = [];
  const projectNames = selectedValues(filters.project_ids, filters.project_id)
    .map((id) => projects.value.find((item) => item.id === id)?.name)
    .filter((name): name is string => Boolean(name));
  const userNames = selectedValues(filters.assignee_ids, filters.assignee_id)
    .map((id) => users.value.find((item) => item.id === id)?.name)
    .filter((name): name is string => Boolean(name));
  const statusNames = selectedValues(filters.statuses, filters.status).map(statusLabel);
  const priorityNames = selectedValues(filters.priorities, filters.priority).map(priorityLabel);
  if (projectNames.length) chips.push({ key: "project_ids", label: `项目：${projectNames.join("、")}` });
  if (statusNames.length) chips.push({ key: "statuses", label: `状态：${statusNames.join("、")}` });
  if (priorityNames.length) chips.push({ key: "priorities", label: `优先级：${priorityNames.join("、")}` });
  if (userNames.length) chips.push({ key: "assignee_ids", label: `负责人：${userNames.join("、")}` });
  if (filters.keyword) chips.push({ key: "keyword", label: `关键词：${filters.keyword}` });
  return chips;
});

function selectedValues<T>(many?: T[], single?: T): T[] {
  if (many?.length) return many;
  return single ? [single] : [];
}

function applyFilters(filters: TaskFilters) {
  store.setFilters(filters);
}

function toggleMyTasksFilter() {
  const userId = currentUser.value?.id;
  if (!userId) return;
  const assigneeIds = selectedValues(store.filters.assignee_ids, store.filters.assignee_id);
  if (assigneeIds.length === 1 && assigneeIds[0] === userId) {
    store.clearFilter("assignee_ids");
    store.clearFilter("assignee_id");
    return;
  }
  store.setFilters({ ...store.filters, assignee_id: undefined, assignee_ids: [userId] });
}

function showAllByStatus(status: "todo" | "in_progress" | "blocked" | "done") {
  store.setFilters({ ...store.filters, status: undefined, statuses: [status] });
  store.setView("list");
}
</script>

<template>
  <div class="workbench-page">
    <section class="filter-strip">
      <ElButton type="primary" plain class="filter-button" @click="showFilters = true">
        <ElIcon><Filter /></ElIcon>
        筛选
      </ElButton>
      <ElButton
        v-if="currentUser"
        :type="isMyTasksFilterActive ? 'primary' : 'default'"
        plain
        class="filter-button"
        @click="toggleMyTasksFilter"
      >
        我的任务
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
      <div v-if="store.view !== 'gantt'" class="view-alert-row">
        <ElAlert
          class="view-alert"
          type="primary"
          show-icon
          :closable="false"
          title="员工仅可拖拽自己负责的任务，管理人员可拖拽全部任务"
        />
      </div>
      <TaskKanbanView
        v-if="store.view === 'kanban' && currentUser"
        :tasks="tasks"
        :current-user="currentUser"
        @show-all="showAllByStatus"
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
