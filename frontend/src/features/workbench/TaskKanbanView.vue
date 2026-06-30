<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { ElMessage } from "element-plus";
import { VueDraggableNext } from "vue-draggable-next";
import { useRouter } from "vue-router";
import { MoreFilled } from "@element-plus/icons-vue";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import type { CurrentUser, DisplayStatus, Task, TaskStatus, User } from "@/types";
import {
  canMoveTaskToStatus,
  displayColumns,
  groupTasksByDisplayStatus,
  mergeTaskPreservingSubtasks,
  moveTaskForDisplay,
  primaryTaskAssigneeName,
  statusLabel,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";
import { kanbanStatusLimit, visibleKanbanTasks } from "./taskKanban";

const props = defineProps<{
  tasks: Task[];
  currentUser: CurrentUser;
  users: User[];
}>();

const emit = defineEmits<{
  showAll: [status: TaskStatus];
}>();

const router = useRouter();
const queryClient = useQueryClient();
const localTasks = ref<Task[]>([]);
const pendingMoves = ref<Record<string, TaskStatus>>({});
const groups = computed(() => groupTasksByDisplayStatus(localTasks.value));

function applyPendingMoves(tasks: Task[]) {
  return Object.entries(pendingMoves.value).reduce(
    (nextTasks, [taskId, status]) => moveTaskForDisplay(nextTasks, taskId, status),
    tasks.map((task) => ({ ...task })),
  );
}

watch(
  () => props.tasks,
  (tasks) => {
    localTasks.value = applyPendingMoves(tasks);
  },
  { immediate: true },
);

const mutation = useMutation({
  mutationFn: ({ task, status }: { task: Task; status: TaskStatus }) =>
    api.updateTaskStatus(task.id, status),
});

function replaceLocalTask(task: Task) {
  localTasks.value = localTasks.value.map((item) =>
    item.id === task.id ? mergeTaskPreservingSubtasks(item, task) : item,
  );
}

function clearPendingMove(taskId: string) {
  const nextPendingMoves = { ...pendingMoves.value };
  delete nextPendingMoves[taskId];
  pendingMoves.value = nextPendingMoves;
}

function primaryTaskAssigneeAvatar(task: Task): string | undefined {
  const userId = task.assignees?.[0]?.id ?? task.assignee_id;
  return props.users.find((user) => user.id === userId)?.avatar_url;
}

async function onDrop(status: DisplayStatus, event: { added?: { element: Task } }) {
  const droppedTask = event.added?.element;
  if (!droppedTask) return;

  const previousTask =
    props.tasks.find((task) => task.id === droppedTask.id) ??
    localTasks.value.find((task) => task.id === droppedTask.id) ??
    droppedTask;

  if (!canMoveTaskToStatus(previousTask, status, props.currentUser)) {
    replaceLocalTask(previousTask);
    if (
      status === "done" &&
      previousTask.subtasks?.some((subtask) => subtask.status !== "done")
    ) {
      ElMessage.warning("完成全部子任务后才能完成主任务");
    } else {
      ElMessage.warning("员工仅可拖拽自己负责的任务，管理人员可拖拽全部任务");
    }
    return;
  }

  pendingMoves.value = { ...pendingMoves.value, [droppedTask.id]: status };
  localTasks.value = moveTaskForDisplay(localTasks.value, droppedTask.id, status);

  try {
    const updatedTask = await mutation.mutateAsync({ task: previousTask, status });
    clearPendingMove(droppedTask.id);
    replaceLocalTask(updatedTask);
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  } catch {
    clearPendingMove(droppedTask.id);
    replaceLocalTask(previousTask);
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  }
}
</script>

<template>
  <div class="kanban-board">
    <section v-for="column in displayColumns" :key="column.key" class="kanban-column">
      <header>
        <span class="column-title">
          <i :class="column.dotClass" />
          {{ column.title }}（{{ groups[column.key].length }}）
        </span>
        <ElIcon><MoreFilled /></ElIcon>
      </header>
      <VueDraggableNext
        class="kanban-list"
        :list="visibleKanbanTasks(column.key, groups[column.key])"
        :group="{ name: 'tasks' }"
        item-key="id"
        @change="onDrop(column.key, $event)"
      >
        <article
          v-for="task in groups[column.key]"
          :key="task.id"
          class="task-card"
          @click="router.push(`/tasks/${task.id}`)"
        >
          <div class="task-card-title">
            <strong>{{ task.title }}</strong>
            <span v-if="task.is_overdue" class="overdue-mark">已延期</span>
          </div>
          <p>所属项目：{{ task.project_name || "-" }}</p>
          <p class="assignee-line">
            负责人：
            <UserAvatar :name="primaryTaskAssigneeName(task)" :src="primaryTaskAssigneeAvatar(task)" :size="22" />
            {{ taskAssigneeNames(task) }}
          </p>
          <p>截止日期：{{ task.due_date }}</p>
          <p>
            优先级：
            <PriorityTag :priority="task.priority" />
          </p>
        </article>
      </VueDraggableNext>
      <button
        v-if="groups[column.key].length > kanbanStatusLimit(column.key)"
        type="button"
        class="kanban-more-button"
        @click="emit('showAll', column.key)"
      >
        查看全部{{ statusLabel(column.key) }}任务
      </button>
    </section>
  </div>
</template>
