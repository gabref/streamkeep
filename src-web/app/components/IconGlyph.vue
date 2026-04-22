<template>
  <svg
    viewBox="0 0 24 24"
    role="img"
    focusable="false"
  >
    <path :d="path" />
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue';

export type IconName =
  | 'activity'
  | 'arrow-left'
  | 'arrow-right'
  | 'bug'
  | 'download'
  | 'folder'
  | 'home'
  | 'play'
  | 'rotate-cw'
  | 'settings'
  | 'shield'
  | 'sliders';

const props = defineProps<{
  name: IconName;
}>();

const paths: Record<IconName, string> = {
  activity: 'M4 12h4l2-6 4 12 2-6h4',
  'arrow-left': 'M14 6l-6 6 6 6M8 12h12',
  'arrow-right': 'M10 6l6 6-6 6m6-6H4',
  bug: 'M8 8h8v9a4 4 0 0 1-8 0V8Zm2-3h4m-7 6H4m16 0h-3M6 16H3m18 0h-3',
  download: 'M12 4v10m0 0 4-4m-4 4-4-4M5 20h14',
  folder: 'M3 7h7l2 2h9v9H3V7Z',
  home: 'M4 11 12 4l8 7v9h-5v-6H9v6H4v-9Z',
  play: 'M7 5v14l12-7L7 5Z',
  'rotate-cw': 'M20 6v5h-5M4 18v-5h5m9.9-4A7 7 0 0 0 6.1 7M5.1 15A7 7 0 0 0 17.9 17',
  settings:
    'M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8Zm0-5v3m0 12v3M4.2 4.2l2.1 2.1m11.4 11.4 2.1 2.1M3 12h3m12 0h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1',
  shield: 'M12 3 5 6v6c0 4.2 2.7 7.2 7 9 4.3-1.8 7-4.8 7-9V6l-7-3Z',
  sliders: 'M5 7h9m3 0h2M5 17h2m3 0h9M9 4v6m6 4v6',
};

const path = computed(() => paths[props.name]);
</script>
