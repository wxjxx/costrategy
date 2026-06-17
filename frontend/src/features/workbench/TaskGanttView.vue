<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { gantt } from "dhtmlx-gantt";
import "dhtmlx-gantt/codebase/dhtmlxgantt.css";
import type { Task } from "@/types";
import {
  buildGanttTasks,
  buildGanttTimelineRange,
  calculateCenteredTimelineScrollX,
  defaultGanttScaleMode,
  formatGanttBarContent,
  formatGanttTodayMarkerText,
  formatGanttTooltip,
  isGanttProjectOpen,
  shouldToggleProjectRowFromClick,
  shouldUseDefaultGanttClick,
  taskDetailPathFromGanttTask,
  type GanttTask,
  type GanttScaleMode,
} from "./taskGantt";

const props = defineProps<{ tasks: Task[] }>();

const router = useRouter();
const container = ref<HTMLElement>();
const taskClickEventId = ref<string>();
const todayMarkerId = ref<string | number>();

const scaleOptions: GanttScaleMode[] = ["按天", "按周"];
const scaleMode = ref<GanttScaleMode>(defaultGanttScaleMode);

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
    gantt.config.scale_unit = "day";
    gantt.config.scales = [{ unit: "day", step: 1, format: formatChineseDayScale }];
    gantt.config.min_column_width = 70;
    return;
  }

  gantt.config.scale_unit = "week";
  gantt.config.scales = [{ unit: "week", step: 1, format: formatChineseWeekScale }];
  gantt.config.min_column_width = 150;
}

function todayForGantt(): Date {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth(), now.getDate());
}

function applyTimelineRange() {
  const range = buildGanttTimelineRange(props.tasks, todayForGantt());
  gantt.config.start_date = range.start_date;
  gantt.config.end_date = range.end_date;
}

function removeTodayMarker() {
  if (todayMarkerId.value === undefined) return;
  gantt.deleteMarker(todayMarkerId.value);
  todayMarkerId.value = undefined;
}

function renderTodayMarker() {
  removeTodayMarker();
  const today = todayForGantt();
  todayMarkerId.value = gantt.addMarker({
    start_date: today,
    css: "gantt-today-marker",
    text: formatGanttTodayMarkerText(today),
    title: formatGanttTodayMarkerText(today),
  });
}

function timelineViewportWidth(): number {
  return (
    container.value?.querySelector<HTMLElement>(".gantt_task")?.clientWidth ?? 0
  );
}

function centerTodayInTimeline() {
  requestAnimationFrame(() => {
    const viewportWidth = timelineViewportWidth();
    if (!viewportWidth) return;

    const scrollState = gantt.getScrollState();
    const todayX = gantt.posFromDate(todayForGantt());
    gantt.scrollTo(
      calculateCenteredTimelineScrollX(todayX, viewportWidth),
      scrollState.y,
    );
  });
}

function render() {
  if (!container.value) return;
  gantt.clearAll();
  todayMarkerId.value = undefined;
  applyTimelineRange();
  renderTodayMarker();
  gantt.parse({
    data: buildGanttTasks(props.tasks, gantt.config.types.project ?? "project"),
  });
  centerTodayInTimeline();
}

function handleGanttGridProjectClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (!target || target.closest(".gantt_tree_icon")) return;

  const cell = target.closest<HTMLElement>(".gantt_grid_data .gantt_cell_tree");
  if (!cell || !container.value?.contains(cell)) return;

  const row = cell.closest<HTMLElement>(".gantt_row[data-task-id], .gantt_row[task_id]");
  const id = row?.dataset.taskId || row?.getAttribute("task_id");
  if (!id || !gantt.isTaskExists(id)) return;

  const task = gantt.getTask(id) as GanttTask;
  if (!shouldToggleProjectRowFromClick(task, false)) return;

  event.preventDefault();
  event.stopPropagation();
  if (isGanttProjectOpen(task)) {
    gantt.close(id);
  } else {
    gantt.open(id);
  }
}

onMounted(() => {
  if (!container.value) return;
  gantt.plugins({ marker: true });
  gantt.config.readonly = true;
  gantt.config.date_format = "%Y-%m-%d";
  gantt.config.initial_scroll = false;
  gantt.config.scale_unit = "week";
  gantt.config.step = 1;
  gantt.config.subscales = [{ unit: "day", step: 1, date: "%m.%d" }];
  applyScaleMode();
  gantt.config.columns = [{ name: "text", label: "任务名称", tree: true, width: 260 }];
  gantt.templates.tooltip_text = (_start, _end, task) =>
    formatGanttTooltip(task as GanttTask);
  gantt.templates.task_text = (_start, _end, task) =>
    formatGanttBarContent(task as GanttTask);
  taskClickEventId.value = gantt.attachEvent("onTaskClick", (id, event) => {
    const task = gantt.getTask(id) as GanttTask;
    const clickedTreeIcon = Boolean(
      (event?.target as HTMLElement | null)?.closest(".gantt_tree_icon"),
    );
    if (shouldToggleProjectRowFromClick(task, clickedTreeIcon)) {
      if (isGanttProjectOpen(task)) {
        gantt.close(id);
      } else {
        gantt.open(id);
      }
      return false;
    }
    if (shouldUseDefaultGanttClick(task)) return true;
    const path = taskDetailPathFromGanttTask(task);
    if (path) void router.push(path);
    return false;
  });
  gantt.init(container.value);
  container.value.addEventListener("click", handleGanttGridProjectClick);
  render();
});

watch(() => props.tasks, render, { deep: true });

watch(scaleMode, () => {
  if (!container.value) return;
  applyScaleMode();
  applyTimelineRange();
  renderTodayMarker();
  gantt.render();
  centerTodayInTimeline();
});

onBeforeUnmount(() => {
  if (taskClickEventId.value) {
    gantt.detachEvent(taskClickEventId.value);
  }
  container.value?.removeEventListener("click", handleGanttGridProjectClick);
  removeTodayMarker();
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
