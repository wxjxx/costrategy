<script setup lang="ts">
import type { TaskPriority } from "@/types";
import { priorityLabel } from "@/features/tasks/taskWorkflow";

defineProps<{
  priority: TaskPriority;
}>();
</script>

<template>
  <span class="priority-tag" :class="`priority-${priority}`">
    {{ priorityLabel(priority) }}
  </span>
</template>
