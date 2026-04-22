<template>
  <span
    class="statusChip"
    :class="`statusChip--${tone}`"
  >
    <slot />
  </span>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: 'default' | 'success' | 'warning' | 'danger';
  }>(),
  {
    tone: 'default',
  }
);
</script>

