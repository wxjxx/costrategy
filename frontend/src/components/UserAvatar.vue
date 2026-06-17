<script setup lang="ts">
defineProps<{
  name?: string;
  src?: string;
  size?: number;
}>();
</script>

<template>
  <span class="user-avatar" :style="{ width: `${size ?? 28}px`, height: `${size ?? 28}px` }">
    <img v-if="src" :src="src" :alt="name ? `${name}头像` : '用户头像'" />
    <template v-else>
      {{ (name || "用").slice(0, 1) }}
    </template>
  </span>
</template>
