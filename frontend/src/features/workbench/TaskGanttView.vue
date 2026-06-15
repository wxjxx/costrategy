<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { gantt } from "dhtmlx-gantt";
import "dhtmlx-gantt/codebase/dhtmlxgantt.css";
import type { Task } from "@/types";
import {
  buildGanttTasks,
  formatGanttTooltip,
  taskDetailPathFromGanttTask,
  type GanttTask,
} from "./taskGantt";

const props = defineProps<{ tasks: Task[] }>();

const router = useRouter();
const container = ref<HTMLElement>();
type GanttScaleMode = "按天" | "按周";
const taskClickEventId = ref<string>();

const scaleOptions: GanttScaleMode[] = ["按天", "按周"];
const scaleMode = ref<GanttScaleMode>("按周");

function formatChineseDayScale(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${month}月${day}日`;
}

function formatChineseWeekScale(date: Date): string {
  const endDate = new Date(date);
  endDate.setDate(date.getDate() + 6);
  return `${formatChineseDayScale(date)}-${formatChineseDayScale(endDate)}`;
}

function applyScaleMode() {
  if (scaleMode.value === "按天") {
    gantt.config.scales = [{ unit: "day", step: 1, format: formatChineseDayScale }];
    gantt.config.min_column_width = 70;
    return;
  }

  gantt.config.scales = [{ unit: "week", step: 1, format: formatChineseWeekScale }];
  gantt.config.min_column_width = 150;
}

function render() {
  if (!container.value) return;
  gantt.clearAll();
  gantt.parse({
    data: buildGanttTasks(props.tasks, gantt.config.types.project ?? "project"),
  });
}

onMounted(() => {
  if (!container.value) return;
  gantt.config.readonly = true;
  gantt.config.date_format = "%Y-%m-%d";
  gantt.config.scale_unit = "week";
  gantt.config.step = 1;
  gantt.config.subscales = [{ unit: "day", step: 1, date: "%m.%d" }];
  applyScaleMode();
  gantt.config.columns = [{ name: "text", label: "任务名称", tree: true, width: 260 }];
  gantt.templates.tooltip_text = (_start, _end, task) =>
    formatGanttTooltip(task as GanttTask);
  taskClickEventId.value = gantt.attachEvent("onTaskClick", (id) => {
    const path = taskDetailPathFromGanttTask(gantt.getTask(id) as GanttTask);
    if (path) void router.push(path);
    return false;
  });
  gantt.init(container.value);
  render();
});

watch(() => props.tasks, render, { deep: true });

watch(scaleMode, () => {
  if (!container.value) return;
  applyScaleMode();
  gantt.render();
});

onBeforeUnmount(() => {
  if (taskClickEventId.value) {
    gantt.detachEvent(taskClickEventId.value);
  }
  gantt.clearAll();
});
</script>

<template>
  <div class="gantt-shell">
    <div class="gantt-toolbar">
      <h2>甘特图视图</h2>
      <div class="gantt-legend">
        <span><i class="legend-todo" />待开始</span>
        <span><i class="legend-in-progress" />进行中</span>
        <span><i class="legend-done" />已完成</span>
        <span><i class="legend-blocked" />阻塞</span>
        <span><i class="legend-overdue" />已延期</span>
      </div>
      <ElSegmented v-model="scaleMode" :options="scaleOptions" />
    </div>
    <div ref="container" class="gantt-container" />
  </div>
</template>
