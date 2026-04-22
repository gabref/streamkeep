<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Embedded player
        </p>
        <h1>Player</h1>
      </div>
      <StatusChip :tone="detectionTone">
        {{ detectionStatus }}
      </StatusChip>
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
        <div class="metricTile">
          <span>Selected file</span>
          <strong>{{ confirmedFileLabel }}</strong>
        </div>
        <div class="metricTile">
          <span>Download</span>
          <strong>{{ downloadProgressLabel }}</strong>
        </div>
      </div>
    </section>

    <DetectionDialog
      v-if="pendingStream"
      :stream="pendingStream"
      @cancel="detection.dismissPending"
      @download="confirmDownload"
    />
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
  listenForDownloadProgress,
  startDownload,
  type DownloadProgressPayload,
  type StartDownloadResult,
} from '@/api/downloads';
import {
  getPlayerState,
  openPlayer,
  playerGoBack,
  playerGoForward,
  playerReload,
  type PlayerState,
} from '@/api/player';
import AppButton from '@/app/components/AppButton.vue';
import DetectionDialog from '@/app/components/DetectionDialog.vue';
import IconGlyph from '@/app/components/IconGlyph.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { useDetectionStore } from '@/stores/detection';

const targetUrl = ref('https://example.com');
const playerState = ref<PlayerState | null>(null);
const lastRequest = ref<CaptureRequestPayload | null>(null);
const lastDetectedMaster = ref<CaptureRequestPayload | null>(null);
const activeDownloadProgress = ref<DownloadProgressPayload | null>(null);
const latestDownloadResult = ref<StartDownloadResult | null>(null);
const errorMessage = ref<string | null>(null);
const isBusy = ref(false);
const listenerCleanup = ref<Array<() => Promise<void>>>([]);
const detection = useDetectionStore();

const pendingStream = computed(() => detection.pendingStream);

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
  if (latestDownloadResult.value) {
    return `Saved ${latestDownloadResult.value.outputName}`;
  }
  if (detection.confirmedIntent) {
    return `Ready to download ${detection.confirmedIntent.fileName}`;
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
  if (pendingStream.value) {
    return 'Confirm download';
  }
  if (lastDetectedMaster.value) {
    return 'Stream detected';
  }
  return 'Idle';
});

const detectionTone = computed<'default' | 'success'>(() => {
  if (pendingStream.value || lastDetectedMaster.value) {
    return 'success';
  }
  return 'default';
});

const lastRequestLabel = computed(() => {
  if (!lastRequest.value) {
    return 'None';
  }
  return `${lastRequest.value.requestType}: ${lastRequest.value.url}`;
});

const confirmedFileLabel = computed(() => {
  return latestDownloadResult.value?.outputName ?? detection.confirmedIntent?.fileName ?? 'None';
});

const downloadProgressLabel = computed(() => {
  if (latestDownloadResult.value) {
    return 'Done';
  }
  if (!activeDownloadProgress.value) {
    return 'Idle';
  }

  const percent = progressPercent(activeDownloadProgress.value);
  if (activeDownloadProgress.value.message && percent === null) {
    return activeDownloadProgress.value.message;
  }
  if (activeDownloadProgress.value.message) {
    return `${activeDownloadProgress.value.message} ${percent}%`;
  }
  if (percent === null) {
    return activeDownloadProgress.value.status;
  }
  return `${activeDownloadProgress.value.status} ${percent}%`;
});

onMounted(async () => {
  const requestListener = await listenForCaptureRequest((payload) => {
    lastRequest.value = payload;
  });
  const masterListener = await listenForMasterDetected((payload) => {
    lastRequest.value = payload;
    lastDetectedMaster.value = payload;
    detection.registerDetectedPayload(payload);
  });
  listenerCleanup.value = [
    () => requestListener.unregister(),
    () => masterListener.unregister(),
  ];
  const downloadProgressListener = await listenForDownloadProgress((payload) => {
    activeDownloadProgress.value = payload;
  });
  listenerCleanup.value.push(async () => {
    downloadProgressListener();
  });
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

async function confirmDownload(fileNameStem: string, qualityId: string) {
  const intent = detection.confirmDownload(fileNameStem, qualityId);
  if (!intent) {
    return;
  }

  isBusy.value = true;
  errorMessage.value = null;
  activeDownloadProgress.value = {
    status: 'queued',
    completedSegments: 0,
    totalSegments: null,
    downloadedBytes: 0,
    totalBytes: null,
    message: 'queued',
  };

  try {
    const selectedQuality = intent.stream.qualities.find((quality) => quality.id === qualityId);
    latestDownloadResult.value = await startDownload({
      masterUrl: intent.stream.masterUrl,
      mediaPlaylistUrl: selectedQuality?.mediaPlaylistUrl ?? null,
      referer: intent.stream.referer,
      userAgent: intent.stream.userAgent,
      cookies: intent.stream.cookies,
      outputName: intent.fileName,
    });
  } catch (error) {
    activeDownloadProgress.value = {
      status: 'failed',
      completedSegments: activeDownloadProgress.value?.completedSegments ?? 0,
      totalSegments: activeDownloadProgress.value?.totalSegments ?? null,
      downloadedBytes: activeDownloadProgress.value?.downloadedBytes ?? 0,
      totalBytes: activeDownloadProgress.value?.totalBytes ?? null,
      message: error instanceof Error ? error.message : String(error),
    };
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isBusy.value = false;
  }
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

function progressPercent(progress: DownloadProgressPayload): number | null {
  if (progress.totalBytes && progress.totalBytes > 0) {
    return Math.floor((Math.min(progress.downloadedBytes, progress.totalBytes) * 100) / progress.totalBytes);
  }
  if (progress.totalSegments && progress.totalSegments > 0) {
    return Math.floor((Math.min(progress.completedSegments, progress.totalSegments) * 100) / progress.totalSegments);
  }
  return null;
}
</script>
