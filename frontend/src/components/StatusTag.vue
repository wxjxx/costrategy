<script setup lang="ts">
import type { DisplayStatus, TaskStatus } from "@/types";
import { statusLabel } from "@/features/tasks/taskWorkflow";

defineProps<{
  status: DisplayStatus | TaskStatus;
}>();
</script>

<template>
  <span class="status-tag" :class="`status-${status}`">
    <i />
    {{ statusLabel(status) }}
  </span>
</template>
