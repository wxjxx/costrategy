<script setup lang="ts">
import { computed } from "vue";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { VueDraggableNext } from "vue-draggable-next";
import { useRouter } from "vue-router";
import { MoreFilled } from "@element-plus/icons-vue";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import type { CurrentUser, DisplayStatus, Task } from "@/types";
import {
  canMoveTaskToStatus,
  displayColumns,
  groupTasksByDisplayStatus,
} from "@/features/tasks/taskWorkflow";

const props = defineProps<{
  tasks: Task[];
  currentUser: CurrentUser;
}>();

const router = useRouter();
const queryClient = useQueryClient();
const groups = computed(() => groupTasksByDisplayStatus(props.tasks));

const mutation = useMutation({
  mutationFn: ({ task, status }: { task: Task; status: DisplayStatus }) =>
    canMoveTaskToStatus(task, status, props.currentUser)
      ? api.updateTaskStatus(task.id, status)
      : Promise.resolve(task),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["tasks"] }),
});

function onDrop(status: DisplayStatus, event: { added?: { element: Task } }) {
  if (event.added) {
    mutation.mutate({ task: event.added.element, status });
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
        :list="groups[column.key]"
        :group="{ name: 'tasks', pull: column.key !== 'overdue', put: column.key !== 'overdue' }"
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
            <span v-if="task.is_overdue && task.status !== 'done'" class="overdue-mark">已延期</span>
          </div>
          <p>所属项目：{{ task.project_name || "-" }}</p>
          <p class="assignee-line">
            负责人：
            <UserAvatar :name="task.assignee_name" :size="22" />
            {{ task.assignee_name || "-" }}
          </p>
          <p>截止日期：{{ task.due_date }}</p>
          <p>
            优先级：
            <PriorityTag :priority="task.priority" />
          </p>
        </article>
      </VueDraggableNext>
    </section>
  </div>
</template>
