<script setup lang="ts">
import Draggable from 'vuedraggable';
import { reactive, watch } from 'vue';
import {
  groupTasksByStatus,
  TASK_STATUS_COLUMNS,
  taskAssigneeLabel,
  taskPriorityLabel,
  taskProjectLabel,
  type TaskItem,
  type TaskStatus,
  type TaskStatusGroups,
} from './taskModel';

const props = withDefaults(
  defineProps<{
    tasks: TaskItem[];
    updatingTaskId?: string;
  }>(),
  {
    updatingTaskId: '',
  },
);

const emit = defineEmits<{
  'open-task': [taskId: string];
  'status-change': [payload: { taskId: string; status: TaskStatus }];
}>();

const columnTasks = reactive<TaskStatusGroups>({
  todo: [],
  in_progress: [],
  done: [],
});

watch(
  () => props.tasks,
  (tasks) => {
    const grouped = groupTasksByStatus(tasks);
    TASK_STATUS_COLUMNS.forEach(({ status }) => {
      columnTasks[status] = [...grouped[status]];
    });
  },
  { immediate: true, deep: true },
);

function handleColumnChange(status: TaskStatus, event: { added?: { element: TaskItem } }) {
  const movedTask = event.added?.element;
  if (!movedTask || movedTask.status === status) {
    return;
  }

  emit('status-change', { taskId: movedTask.id, status });
}

defineExpose({ handleColumnChange });
</script>

<template>
  <div class="task-board" data-test="task-board">
    <section
      v-for="column in TASK_STATUS_COLUMNS"
      :key="column.status"
      class="task-column"
      :data-test="`task-column-${column.status}`"
    >
      <header class="task-column__header">
        <strong>{{ column.label }}</strong>
        <span>{{ columnTasks[column.status].length }}</span>
      </header>

      <Draggable
        v-model="columnTasks[column.status]"
        class="task-column__list"
        group="task-status"
        item-key="id"
        ghost-class="task-card--ghost"
        @change="handleColumnChange(column.status, $event)"
      >
        <template #item="{ element }">
          <article
            class="task-card"
            :class="{
              'task-card--overdue': element.is_overdue,
              'task-card--updating': updatingTaskId === element.id,
            }"
            :data-test="`task-card-${element.id}`"
            role="button"
            tabindex="0"
            @click="emit('open-task', element.id)"
            @keydown.enter="emit('open-task', element.id)"
          >
            <div class="task-card__top">
              <h3>{{ element.title }}</h3>
              <span class="priority-badge" :data-priority="element.priority">
                {{ taskPriorityLabel(element.priority) }}
              </span>
            </div>
            <dl class="task-card__meta">
              <div>
                <dt>项目</dt>
                <dd>{{ taskProjectLabel(element) }}</dd>
              </div>
              <div>
                <dt>负责人</dt>
                <dd>{{ taskAssigneeLabel(element) }}</dd>
              </div>
              <div>
                <dt>截止</dt>
                <dd>{{ element.due_date }}</dd>
              </div>
            </dl>
            <div v-if="element.is_overdue" class="task-card__flag">已延期</div>
          </article>
        </template>
      </Draggable>
    </section>
  </div>
</template>
