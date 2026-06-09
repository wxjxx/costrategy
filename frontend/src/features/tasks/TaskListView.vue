<script setup lang="ts">
import {
  taskAssigneeLabel,
  taskPriorityLabel,
  taskProjectLabel,
  taskStatusLabel,
  type TaskItem,
} from './taskModel';

defineProps<{
  tasks: TaskItem[];
}>();

const emit = defineEmits<{
  'open-task': [taskId: string];
}>();
</script>

<template>
  <div class="task-list" data-test="task-list">
    <table>
      <thead>
        <tr>
          <th>任务标题</th>
          <th>所属项目</th>
          <th>负责人</th>
          <th>状态</th>
          <th>优先级</th>
          <th>截止日期</th>
          <th>延期</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="task in tasks"
          :key="task.id"
          :data-test="`task-row-${task.id}`"
          @click="emit('open-task', task.id)"
        >
          <td>{{ task.title }}</td>
          <td>{{ taskProjectLabel(task) }}</td>
          <td>{{ taskAssigneeLabel(task) }}</td>
          <td>{{ taskStatusLabel(task.status) }}</td>
          <td>{{ taskPriorityLabel(task.priority) }}</td>
          <td>{{ task.due_date }}</td>
          <td>{{ task.is_overdue ? '已延期' : '-' }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
