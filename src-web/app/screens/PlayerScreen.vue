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
        icon="arrow-right"
        :disabled="isBusy"
        @click="runPlayerCommand(goForward)"
      >
        Forward
      </AppButton>
      <AppButton
        icon="rotate-cw"
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
      <p
        v-if="errorMessage"
        class="inlineError"
      >
        {{ errorMessage }}
      </p>
    </section>

    <section class="sectionBlock sectionBlock--compact">
      <h2>Detection state</h2>
      <div class="detectionStrip">
        <div>
          <span>Status</span>
          <strong>{{ detectionStatus }}</strong>
        </div>
        <div>
          <span>Last request</span>
          <strong>{{ lastRequestLabel }}</strong>
        </div>
        <div>
          <span>Player</span>
          <strong>{{ playerStateLabel }}</strong>
        </div>
        <div>
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
  listenForCaptureDownloadRequested,
  listenForCaptureRequest,
  listenForMasterDetected,
  type CaptureDownloadRequestPayload,
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
import StatusChip from '@/app/components/StatusChip.vue';
import { useDetectionStore } from '@/stores/detection';
import { useDownloadsStore } from '@/stores/downloads';
import { clearActiveTextInteraction } from '@/utils/dom';

const LAST_PLAYER_URL_KEY = 'streamkeep:last-player-url';

const targetUrl = ref(readLastPlayerUrl());
const playerState = ref<PlayerState | null>(null);
const lastRequest = ref<CaptureRequestPayload | null>(null);
const lastDetectedMaster = ref<CaptureRequestPayload | null>(null);
const activeDownloadProgress = ref<DownloadProgressPayload | null>(null);
const latestDownloadResult = ref<StartDownloadResult | null>(null);
const errorMessage = ref<string | null>(null);
const isBusy = ref(false);
const listenerCleanup = ref<Array<() => Promise<void>>>([]);
const detection = useDetectionStore();
const downloads = useDownloadsStore();

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
    persistPlayerUrl(payload.pageUrl);
  });
  const masterListener = await listenForMasterDetected((payload) => {
    lastRequest.value = payload;
    lastDetectedMaster.value = payload;
    persistPlayerUrl(payload.pageUrl);
    detection.registerDetectedPayload(payload);
  });
  listenerCleanup.value = [
    () => requestListener.unregister(),
    () => masterListener.unregister(),
  ];
  const nativeDownloadListener = await listenForCaptureDownloadRequested((payload) => {
    applyDetectedPayload(payload);
    void confirmDownload(payload.requestedFileNameStem, 'best-available');
  });
  listenerCleanup.value.push(() => nativeDownloadListener.unregister());
  const downloadProgressListener = await listenForDownloadProgress((payload) => {
    activeDownloadProgress.value = payload;
  });
  listenerCleanup.value.push(async () => {
    downloadProgressListener();
  });
  await runPlayerCommand(async () => {
    const state = await getPlayerState();
    persistPlayerUrl(state.url);
    return state;
  });
});

onBeforeUnmount(() => {
  for (const cleanup of listenerCleanup.value) {
    void cleanup();
  }
});

async function openPlayerUrl() {
  clearActiveTextInteraction();
  persistPlayerUrl(targetUrl.value);
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
  clearActiveTextInteraction();
  const intent = detection.confirmDownload(fileNameStem, qualityId);
  if (!intent) {
    return;
  }

  isBusy.value = true;
  errorMessage.value = null;
  activeDownloadProgress.value = {
    jobId: 'pending',
    status: 'queued',
    completedSegments: 0,
    totalSegments: null,
    currentSegmentIndex: null,
    currentSegmentDownloadedBytes: null,
    currentSegmentTotalBytes: null,
    downloadedBytes: 0,
    totalBytes: null,
    message: 'queued',
  };

  try {
    const selectedQuality = intent.stream.qualities.find((quality) => quality.id === qualityId);
    const detectedMediaPlaylist =
      lastRequest.value?.requestType === 'playlist' && /\.m3u8($|\?)/i.test(lastRequest.value.url)
        ? lastRequest.value.url
        : null;
    latestDownloadResult.value = await startDownload({
      masterUrl: intent.stream.masterUrl,
      mediaPlaylistUrl: selectedQuality?.mediaPlaylistUrl ?? detectedMediaPlaylist,
      referer: intent.stream.referer,
      userAgent: intent.stream.userAgent,
      cookies: intent.stream.cookies,
      outputName: intent.fileName,
      title: intent.stream.titleSuggestion,
      pageUrl: intent.stream.pageUrl,
      qualityLabel: selectedQuality?.label ?? 'Best available',
    });
    await downloads.loadHistory();
  } catch (error) {
    activeDownloadProgress.value = {
      jobId: activeDownloadProgress.value?.jobId ?? 'pending',
      status: 'failed',
      completedSegments: activeDownloadProgress.value?.completedSegments ?? 0,
      totalSegments: activeDownloadProgress.value?.totalSegments ?? null,
      currentSegmentIndex: activeDownloadProgress.value?.currentSegmentIndex ?? null,
      currentSegmentDownloadedBytes: activeDownloadProgress.value?.currentSegmentDownloadedBytes ?? null,
      currentSegmentTotalBytes: activeDownloadProgress.value?.currentSegmentTotalBytes ?? null,
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
    persistPlayerUrl(playerState.value.url);
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
    const currentSegmentProgress =
      progress.currentSegmentDownloadedBytes && progress.currentSegmentTotalBytes
        ? Math.min(progress.currentSegmentDownloadedBytes, progress.currentSegmentTotalBytes) /
          progress.currentSegmentTotalBytes
        : 0;
    const completedSegments = Math.min(progress.completedSegments, progress.totalSegments);
    return Math.floor(((completedSegments + currentSegmentProgress) * 100) / progress.totalSegments);
  }
  return null;
}

function applyDetectedPayload(payload: CaptureRequestPayload | CaptureDownloadRequestPayload) {
  lastRequest.value = payload;
  lastDetectedMaster.value = payload;
  persistPlayerUrl(payload.pageUrl);
  detection.registerDetectedPayload(payload);
}

function readLastPlayerUrl(): string {
  return globalThis.localStorage?.getItem(LAST_PLAYER_URL_KEY) ?? 'https://example.com';
}

function persistPlayerUrl(value: string | null | undefined) {
  const normalized = value?.trim();
  if (!normalized) {
    return;
  }
  targetUrl.value = normalized;
  globalThis.localStorage?.setItem(LAST_PLAYER_URL_KEY, normalized);
}

</script>
