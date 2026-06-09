<script setup lang="ts">
import DhtmlxGantt from './DhtmlxGantt.vue';
import type { TaskItem } from './taskModel';

defineProps<{
  tasks: TaskItem[];
}>();
</script>

<template>
  <section class="task-gantt" data-test="task-gantt">
    <DhtmlxGantt :tasks="tasks" />
  </section>
</template>
