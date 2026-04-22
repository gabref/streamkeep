<template>
  <div
    class="progressBar"
    :aria-label="label"
    role="progressbar"
    :aria-valuenow="value"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <span :style="{ width: `${boundedValue}%` }" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    label?: string;
    value: number;
  }>(),
  {
    label: 'Progress',
  }
);

const boundedValue = computed(() => Math.max(0, Math.min(100, props.value)));
</script>

