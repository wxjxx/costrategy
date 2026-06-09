<script setup lang="ts">
defineProps<{
  name?: string;
  size?: number;
}>();
</script>

<template>
  <span class="user-avatar" :style="{ width: `${size ?? 28}px`, height: `${size ?? 28}px` }">
    {{ (name || "用").slice(0, 1) }}
  </span>
</template>
