<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { gantt } from "dhtmlx-gantt";
import "dhtmlx-gantt/codebase/dhtmlxgantt.css";
import type { Task } from "@/types";

const props = defineProps<{ tasks: Task[] }>();

const container = ref<HTMLElement>();

function colorForStatus(task: Task): string {
  if (task.status !== "done" && task.is_overdue) return "#ff4d4f";
  if (task.status === "done") return "#8b95a5";
  if (task.status === "in_progress") return "#18a957";
  return "#0f6bff";
}

function render() {
  if (!container.value) return;
  gantt.clearAll();
  gantt.parse({
    data: props.tasks.map((task) => ({
      id: task.id,
      text: `${task.title}  ${task.assignee_name ?? ""}`,
      start_date: task.start_date,
      end_date: task.due_date,
      color: colorForStatus(task),
    })),
  });
}

onMounted(() => {
  if (!container.value) return;
  gantt.config.readonly = true;
  gantt.config.date_format = "%Y-%m-%d";
  gantt.config.scale_unit = "week";
  gantt.config.step = 1;
  gantt.config.subscales = [{ unit: "day", step: 1, date: "%m.%d" }];
  gantt.config.columns = [{ name: "text", label: "任务名称", tree: true, width: 260 }];
  gantt.templates.tooltip_text = (_start, _end, task) => `<b>${task.text}</b>`;
  gantt.init(container.value);
  render();
});

watch(() => props.tasks, render, { deep: true });

onBeforeUnmount(() => {
  gantt.clearAll();
});
</script>

<template>
  <div class="gantt-shell">
    <div class="gantt-toolbar">
      <h2>甘特图视图</h2>
      <div class="gantt-legend">
        <span><i class="legend-blue" />待开始</span>
        <span><i class="legend-green" />进行中</span>
        <span><i class="legend-gray" />已完成</span>
        <span><i class="legend-red" />已延期</span>
      </div>
      <ElSegmented :options="['按天', '按周']" model-value="按周" />
    </div>
    <div ref="container" class="gantt-container" />
    <ElAlert
      class="view-alert"
      type="primary"
      show-icon
      :closable="false"
      title="第一版仅支持查看，不支持拖拽改期和依赖线。"
    />
  </div>
</template>
