<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, reactive, ref } from 'vue';
import { ElMessage } from 'element-plus';
import { Plus } from 'lucide-vue-next';
import { getHttpErrorMessage } from '../api/http';
import TaskDetailPanel from '../features/tasks/TaskDetailPanel.vue';
import TaskKanbanView from '../features/tasks/TaskKanbanView.vue';
import TaskListView from '../features/tasks/TaskListView.vue';
import {
  createTask,
  createTaskComment,
  deleteTaskAttachment,
  fetchTaskDetail,
  fetchTasks,
  uploadTaskAttachment,
  updateTask,
  updateTaskStatus,
} from '../features/tasks/taskApi';
import {
  countActiveTaskFilters,
  replaceTaskStatus,
  TASK_PRIORITY_OPTIONS,
  TASK_STATUS_COLUMNS,
  type TaskDetail,
  type TaskFilters,
  type TaskItem,
  type TaskStatus,
  type CreateTaskPayload,
  type UpdateTaskPayload,
} from '../features/tasks/taskModel';

type WorkbenchView = 'kanban' | 'list' | 'gantt';

const TaskGanttView = defineAsyncComponent(() => import('../features/tasks/TaskGanttView.vue'));
const activeView = ref<WorkbenchView>('kanban');
const isFilterPanelOpen = ref(false);
const tasks = ref<TaskItem[]>([]);
const isLoading = ref(false);
const updatingTaskId = ref('');
const selectedTaskId = ref('');
const taskDetail = ref<TaskDetail | null>(null);
const isCreatingTask = ref(false);
const isDetailLoading = ref(false);
const isSubmittingComment = ref(false);
const isSavingTask = ref(false);
const isUploadingAttachment = ref(false);
const deletingAttachmentId = ref('');
const filters = reactive<TaskFilters>({
  keyword: '',
  project_id: '',
  assignee_id: '',
  status: '',
  priority: '',
});

const activeFilterCount = computed(() => countActiveTaskFilters(filters));
const projectFilterText = computed(() => (filters.project_id ?? '').trim() || '全部项目');
const assigneeFilterText = computed(() => (filters.assignee_id ?? '').trim() || '全部人员');
const statusFilterText = computed(() => {
  if (!filters.status) {
    return '全部状态';
  }

  return (
    TASK_STATUS_COLUMNS.find((column) => column.status === filters.status)?.label ??
    filters.status
  );
});
const priorityFilterText = computed(() => {
  if (!filters.priority) {
    return '全部优先级';
  }

  return (
    TASK_PRIORITY_OPTIONS.find((option) => option.priority === filters.priority)?.label ??
    filters.priority
  );
});

onMounted(() => {
  void loadTasks();
});

async function loadTasks() {
  isLoading.value = true;

  try {
    tasks.value = await fetchTasks(filters);
  } catch (error) {
    showHttpError(error);
    tasks.value = [];
  } finally {
    isLoading.value = false;
  }
}

async function submitFilters() {
  await loadTasks();
  isFilterPanelOpen.value = false;
}

async function openTaskDetail(taskId: string) {
  isFilterPanelOpen.value = false;
  isCreatingTask.value = false;
  selectedTaskId.value = taskId;
  taskDetail.value = null;
  isDetailLoading.value = true;

  try {
    taskDetail.value = await fetchTaskDetail(taskId);
  } catch (error) {
    showHttpError(error);
    selectedTaskId.value = '';
  } finally {
    isDetailLoading.value = false;
  }
}

function openCreateTask() {
  isFilterPanelOpen.value = false;
  selectedTaskId.value = '';
  isCreatingTask.value = true;
  taskDetail.value = createDraftTaskDetail();
}

function closeTaskDetail() {
  isCreatingTask.value = false;
  selectedTaskId.value = '';
  taskDetail.value = null;
}

async function handleSubmitComment(content: string) {
  if (!selectedTaskId.value || !taskDetail.value) {
    return;
  }

  isSubmittingComment.value = true;

  try {
    const comment = await createTaskComment(selectedTaskId.value, content);
    taskDetail.value = {
      ...taskDetail.value,
      comments: [...taskDetail.value.comments, comment],
    };
  } catch (error) {
    showHttpError(error);
  } finally {
    isSubmittingComment.value = false;
  }
}

async function handleSaveTask(payload: UpdateTaskPayload) {
  if (!taskDetail.value) {
    return;
  }

  isSavingTask.value = true;

  try {
    if (isCreatingTask.value) {
      const created = await createTask(payload as CreateTaskPayload);
      tasks.value = [created, ...tasks.value];
      selectedTaskId.value = created.id;
      isCreatingTask.value = false;
      taskDetail.value = emptyTaskDetail(created);
      return;
    }

    if (!selectedTaskId.value) {
      return;
    }

    const updated = await updateTask(selectedTaskId.value, payload);
    tasks.value = tasks.value.map((task) => (task.id === updated.id ? updated : task));
    taskDetail.value = {
      ...taskDetail.value,
      task: updated,
    };
  } catch (error) {
    showHttpError(error);
  } finally {
    isSavingTask.value = false;
  }
}

async function handleUploadAttachment(file: File) {
  if (!selectedTaskId.value || !taskDetail.value) {
    return;
  }

  isUploadingAttachment.value = true;

  try {
    const attachment = await uploadTaskAttachment(selectedTaskId.value, file);
    taskDetail.value = {
      ...taskDetail.value,
      attachments: [...taskDetail.value.attachments, attachment],
    };
  } catch (error) {
    showHttpError(error);
  } finally {
    isUploadingAttachment.value = false;
  }
}

async function handleDeleteAttachment(attachmentId: string) {
  if (!selectedTaskId.value || !taskDetail.value) {
    return;
  }

  deletingAttachmentId.value = attachmentId;

  try {
    await deleteTaskAttachment(selectedTaskId.value, attachmentId);
    taskDetail.value = {
      ...taskDetail.value,
      attachments: taskDetail.value.attachments.filter(
        (attachment) => attachment.id !== attachmentId,
      ),
    };
  } catch (error) {
    showHttpError(error);
  } finally {
    deletingAttachmentId.value = '';
  }
}

async function handleStatusChange(payload: { taskId: string; status: TaskStatus }) {
  updatingTaskId.value = payload.taskId;
  tasks.value = replaceTaskStatus(tasks.value, payload.taskId, payload.status);

  try {
    const updated = await updateTaskStatus(payload.taskId, payload.status);
    tasks.value = tasks.value.map((task) => (task.id === updated.id ? updated : task));
  } catch (error) {
    showHttpError(error);
    await loadTasks();
  } finally {
    updatingTaskId.value = '';
  }
}

function resetFilters() {
  filters.keyword = '';
  filters.project_id = '';
  filters.assignee_id = '';
  filters.status = '';
  filters.priority = '';
  void loadTasks();
  isFilterPanelOpen.value = false;
}

function showHttpError(error: unknown) {
  ElMessage.error(getHttpErrorMessage(error));
}

function createDraftTaskDetail(): TaskDetail {
  const today = new Date().toISOString().slice(0, 10);
  return emptyTaskDetail({
    id: 'new-task',
    project_id: '',
    title: '',
    assignee_id: '',
    status: 'todo',
    priority: 'medium',
    start_date: today,
    due_date: today,
    description_json: {
      type: 'doc',
      content: [{ type: 'paragraph', content: [] }],
    },
    creator_id: '',
    archived: false,
    is_overdue: false,
    display_status: 'todo',
  });
}

function emptyTaskDetail(task: TaskItem): TaskDetail {
  return {
    task,
    comments: [],
    attachments: [],
    activity_logs: [],
  };
}
</script>

<template>
  <section class="workbench-page">
    <header class="workbench-filter-bar">
      <button
        type="button"
        class="filter-trigger"
        data-test="filter-toggle"
        @click="isFilterPanelOpen = !isFilterPanelOpen"
      >
        筛选
        <span v-if="activeFilterCount > 0">{{ activeFilterCount }}</span>
      </button>
      <button type="button" class="filter-chip" @click="isFilterPanelOpen = true">
        {{ projectFilterText }}
      </button>
      <button type="button" class="filter-chip" @click="isFilterPanelOpen = true">
        {{ assigneeFilterText }}
      </button>
      <button type="button" class="filter-chip" @click="isFilterPanelOpen = true">
        {{ statusFilterText }}
      </button>
      <button type="button" class="filter-chip" @click="isFilterPanelOpen = true">
        {{ priorityFilterText }}
      </button>
    </header>

    <section class="workbench-main-panel">
      <header class="workbench-main-panel__header">
        <div class="view-tabs" aria-label="任务视图">
          <button
            type="button"
            class="view-tab"
            :class="{ 'view-tab--active': activeView === 'kanban' }"
            data-test="view-tab-kanban"
            @click="activeView = 'kanban'"
          >
            看板
          </button>
          <button
            type="button"
            class="view-tab"
            :class="{ 'view-tab--active': activeView === 'list' }"
            data-test="view-tab-list"
            @click="activeView = 'list'"
          >
            列表
          </button>
          <button
            type="button"
            class="view-tab"
            :class="{ 'view-tab--active': activeView === 'gantt' }"
            data-test="view-tab-gantt"
            @click="activeView = 'gantt'"
          >
            甘特图
          </button>
        </div>
        <button
          type="button"
          class="primary-button workbench-create-button"
          data-test="create-task-button"
          @click="openCreateTask"
        >
          <Plus :size="16" />
          新增任务
        </button>
      </header>

      <form
        v-if="isFilterPanelOpen"
        class="task-filters"
        data-test="task-filters"
        @submit.prevent="submitFilters"
      >
        <div class="task-filters__header">
          <strong>筛选任务</strong>
          <button
            type="button"
            class="icon-text-button"
            data-test="filter-close"
            @click="isFilterPanelOpen = false"
          >
            关闭
          </button>
        </div>
        <label>
          <span>关键词</span>
          <input
            v-model="filters.keyword"
            data-test="filter-keyword"
            type="search"
            placeholder="任务标题"
          />
        </label>
        <label>
          <span>项目</span>
          <input v-model="filters.project_id" type="text" placeholder="项目 ID" />
        </label>
        <label>
          <span>人员</span>
          <input v-model="filters.assignee_id" type="text" placeholder="人员 ID" />
        </label>
        <label>
          <span>状态</span>
          <select v-model="filters.status" data-test="filter-status">
            <option value="">全部</option>
            <option
              v-for="column in TASK_STATUS_COLUMNS"
              :key="column.status"
              :value="column.status"
            >
              {{ column.label }}
            </option>
          </select>
        </label>
        <label>
          <span>优先级</span>
          <select v-model="filters.priority">
            <option value="">全部</option>
            <option
              v-for="option in TASK_PRIORITY_OPTIONS"
              :key="option.priority"
              :value="option.priority"
            >
              {{ option.label }}
            </option>
          </select>
        </label>
        <div class="task-filters__actions">
          <button type="button" class="secondary-button" @click="resetFilters">重置</button>
          <button type="submit" class="primary-button" data-test="filter-submit">筛选</button>
        </div>
      </form>

      <div v-if="isLoading" class="feedback" data-test="task-loading">加载中</div>

      <div v-if="!isLoading && tasks.length === 0" class="empty-state" data-test="task-empty">
        暂无任务
      </div>

      <TaskKanbanView
        v-if="activeView === 'kanban' && tasks.length > 0"
        :tasks="tasks"
        :updating-task-id="updatingTaskId"
        @open-task="openTaskDetail"
        @status-change="handleStatusChange"
      />
      <TaskListView
        v-if="activeView === 'list' && tasks.length > 0"
        :tasks="tasks"
        @open-task="openTaskDetail"
      />
      <TaskGanttView v-if="activeView === 'gantt' && tasks.length > 0" :tasks="tasks" />
    </section>

    <TaskDetailPanel
      v-if="selectedTaskId || isCreatingTask"
      :detail="taskDetail"
      :create-mode="isCreatingTask"
      :initial-editing="isCreatingTask"
      :loading="isDetailLoading"
      :submitting="isSubmittingComment"
      :saving-task="isSavingTask"
      :uploading-attachment="isUploadingAttachment"
      :deleting-attachment-id="deletingAttachmentId"
      @close="closeTaskDetail"
      @submit-comment="handleSubmitComment"
      @save-task="handleSaveTask"
      @upload-attachment="handleUploadAttachment"
      @delete-attachment="handleDeleteAttachment"
    />
  </section>
</template>
