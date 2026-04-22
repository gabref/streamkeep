<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Embedded player
        </p>
        <h1>Player</h1>
      </div>
      <StatusChip>Listening idle</StatusChip>
    </div>

    <section class="toolBand">
      <AppButton
        icon="arrow-left"
        :disabled="isBusy"
        @click="runPlayerCommand(goBack)"
      >
        Back
      </AppButton>
      <AppButton
        icon="activity"
        :disabled="isBusy"
        @click="runPlayerCommand(goForward)"
      >
        Forward
      </AppButton>
      <AppButton
        icon="activity"
        :disabled="isBusy"
        @click="runPlayerCommand(reloadPlayer)"
      >
        Reload
      </AppButton>
      <AppButton
        icon="home"
        to="/"
      >
        Home
      </AppButton>
      <AppButton
        icon="download"
        to="/detection"
        variant="primary"
      >
        Detection preview
      </AppButton>
    </section>

    <section class="browserFrame">
      <form
        class="browserFrame__bar"
        @submit.prevent="openPlayerUrl"
      >
        <input
          v-model="targetUrl"
          aria-label="Player URL"
          class="field"
          autocomplete="off"
          autocapitalize="off"
          inputmode="url"
          spellcheck="false"
          type="url"
        >
        <AppButton
          icon="play"
          type="submit"
          variant="primary"
          :disabled="isBusy"
        >
          Open
        </AppButton>
      </form>
      <div class="browserFrame__body">
        <IconGlyph name="shield" />
        <p>{{ playerMessage }}</p>
      </div>
    </section>

    <section class="sectionBlock">
      <h2>Detection state</h2>
      <div class="metricGrid">
        <div class="metricTile">
          <span>Status</span>
          <strong>{{ detectionStatus }}</strong>
        </div>
        <div class="metricTile">
          <span>Last request</span>
          <strong>{{ lastRequestLabel }}</strong>
        </div>
        <div class="metricTile">
          <span>Player</span>
          <strong>{{ playerStateLabel }}</strong>
        </div>
        <div class="metricTile">
          <span>Current page</span>
          <strong>{{ playerState?.title ?? 'None' }}</strong>
        </div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  listenForCaptureRequest,
  listenForMasterDetected,
  type CaptureRequestPayload,
} from '@/api/capture';
import {
  getPlayerState,
  openPlayer,
  playerGoBack,
  playerGoForward,
  playerReload,
  type PlayerState,
} from '@/api/player';
import AppButton from '@/app/components/AppButton.vue';
import IconGlyph from '@/app/components/IconGlyph.vue';
import StatusChip from '@/app/components/StatusChip.vue';

const targetUrl = ref('https://example.com');
const playerState = ref<PlayerState | null>(null);
const lastRequest = ref<CaptureRequestPayload | null>(null);
const lastDetectedMaster = ref<CaptureRequestPayload | null>(null);
const errorMessage = ref<string | null>(null);
const isBusy = ref(false);
const listenerCleanup = ref<Array<() => Promise<void>>>([]);

const playerStateLabel = computed(() => {
  if (!playerState.value) {
    return 'Unknown';
  }
  if (!playerState.value.supported) {
    return 'Unavailable';
  }
  if (playerState.value.visible) {
    return playerState.value.loading ? 'Loading' : 'Open';
  }
  return 'Closed';
});

const playerMessage = computed(() => {
  if (errorMessage.value) {
    return errorMessage.value;
  }
  if (lastDetectedMaster.value?.masterUrl) {
    return lastDetectedMaster.value.masterUrl;
  }
  if (playerState.value?.visible && playerState.value.url) {
    return playerState.value.url;
  }
  return 'Open the Android player to browse and sign in inside Streamkeep.';
});

const detectionStatus = computed(() => {
  if (lastDetectedMaster.value) {
    return 'Stream detected';
  }
  return 'Idle';
});

const lastRequestLabel = computed(() => {
  if (!lastRequest.value) {
    return 'None';
  }
  return `${lastRequest.value.requestType}: ${lastRequest.value.url}`;
});

onMounted(async () => {
  const requestListener = await listenForCaptureRequest((payload) => {
    lastRequest.value = payload;
  });
  const masterListener = await listenForMasterDetected((payload) => {
    lastRequest.value = payload;
    lastDetectedMaster.value = payload;
  });
  listenerCleanup.value = [
    () => requestListener.unregister(),
    () => masterListener.unregister(),
  ];
  await runPlayerCommand(getPlayerState);
});

onBeforeUnmount(() => {
  for (const cleanup of listenerCleanup.value) {
    void cleanup();
  }
});

async function openPlayerUrl() {
  await runPlayerCommand(() => openPlayer(targetUrl.value));
}

async function goBack() {
  return playerGoBack();
}

async function goForward() {
  return playerGoForward();
}

async function reloadPlayer() {
  return playerReload();
}

async function runPlayerCommand(command: () => Promise<PlayerState>) {
  isBusy.value = true;
  errorMessage.value = null;
  try {
    playerState.value = await command();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isBusy.value = false;
  }
}
</script>
