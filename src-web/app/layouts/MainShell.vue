<template>
  <div class="mainShell">
    <header class="mainShell__header">
      <RouterLink
        class="mainShell__brand"
        to="/"
      >
        <img
          alt=""
          aria-hidden="true"
          class="mainShell__mark"
          src="/streamkeep-icon.svg"
        >
        <span>Streamkeep</span>
      </RouterLink>
      <nav
        class="mainShell__topNav"
        aria-label="Primary"
      >
        <RouterLink to="/player">
          Player
        </RouterLink>
        <RouterLink to="/downloads">
          Downloads
        </RouterLink>
        <RouterLink to="/settings">
          Settings
        </RouterLink>
      </nav>
    </header>

    <main class="mainShell__content">
      <slot />
    </main>

    <nav
      class="mainShell__bottomNav"
      aria-label="Primary"
    >
      <RouterLink to="/">
        <IconGlyph name="home" />
        <span>Home</span>
      </RouterLink>
      <RouterLink to="/player">
        <IconGlyph name="play" />
        <span>Player</span>
      </RouterLink>
      <RouterLink to="/downloads">
        <IconGlyph name="download" />
        <span>Downloads</span>
      </RouterLink>
      <RouterLink to="/settings">
        <IconGlyph name="settings" />
        <span>Settings</span>
      </RouterLink>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import {
  listenForDownloadHistoryUpdates,
  listenForDownloadProgress,
  type DownloadProgressPayload,
} from '@/api/downloads';
import IconGlyph from '@/app/components/IconGlyph.vue';
import { useDownloadsStore } from '@/stores/downloads';

const downloads = useDownloadsStore();
const listenerCleanup = ref<Array<() => void>>([]);

onMounted(async () => {
  const progressListener = await listenForDownloadProgress((payload: DownloadProgressPayload) => {
    downloads.applyProgress(payload);
  });
  const historyListener = await listenForDownloadHistoryUpdates((payload) => {
    downloads.upsertRecord(payload);
  });
  listenerCleanup.value = [progressListener, historyListener];
});

onBeforeUnmount(() => {
  for (const cleanup of listenerCleanup.value) {
    cleanup();
  }
});
</script>
