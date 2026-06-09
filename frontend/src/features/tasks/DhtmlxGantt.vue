<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { gantt, type Task as DhtmlxTask } from 'dhtmlx-gantt';
import 'dhtmlx-gantt/codebase/dhtmlxgantt.css';
import { taskAssigneeLabel, taskPriorityLabel, taskStatusLabel, type TaskItem } from './taskModel';

const props = defineProps<{
  tasks: TaskItem[];
}>();

const ganttContainer = ref<HTMLDivElement | null>(null);
let initialized = false;

onMounted(() => {
  if (!ganttContainer.value) {
    return;
  }

  gantt.config.readonly = true;
  gantt.config.drag_move = false;
  gantt.config.drag_resize = false;
  gantt.config.drag_progress = false;
  gantt.config.columns = [
    { name: 'text', label: '任务', tree: true, width: 220 },
    { name: 'owner', label: '负责人', align: 'left', width: 90 },
  ];
  gantt.templates.task_class = (_start: Date, _end: Date, task: DhtmlxTask) =>
    `gantt-task--${task.status}`;
  gantt.templates.tooltip_text = (_start: Date, _end: Date, task: DhtmlxTask) =>
    [
      `<b>${task.text}</b>`,
      `负责人：${task.owner}`,
      `状态：${task.statusLabel}`,
      `优先级：${task.priorityLabel}`,
      `截止：${task.dueDate}`,
    ].join('<br/>');

  gantt.init(ganttContainer.value);
  initialized = true;
  renderGantt();
});

watch(
  () => props.tasks,
  () => renderGantt(),
  { deep: true },
);

onBeforeUnmount(() => {
  if (initialized) {
    gantt.clearAll();
  }
});

function renderGantt() {
  if (!initialized) {
    return;
  }

  gantt.clearAll();
  gantt.parse({
    data: props.tasks.map((task) => ({
      id: task.id,
      text: task.title,
      start_date: toDhtmlxDate(task.start_date),
      end_date: toDhtmlxDate(addOneDay(task.due_date)),
      progress: task.status === 'done' ? 1 : task.status === 'in_progress' ? 0.5 : 0,
      owner: taskAssigneeLabel(task),
      status: task.status,
      statusLabel: taskStatusLabel(task.status),
      priorityLabel: taskPriorityLabel(task.priority),
      dueDate: task.due_date,
      open: true,
    })),
    links: [],
  });
}

function toDhtmlxDate(value: string): string {
  const [year, month, day] = value.split('-');
  return `${day}-${month}-${year}`;
}

function addOneDay(value: string): string {
  const date = new Date(`${value}T00:00:00`);
  date.setDate(date.getDate() + 1);
  return date.toISOString().slice(0, 10);
}
</script>

<template>
  <div ref="ganttContainer" class="dhtmlx-gantt" data-test="dhtmlx-gantt" />
</template>
