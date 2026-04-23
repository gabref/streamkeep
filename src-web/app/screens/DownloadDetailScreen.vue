<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Download detail
        </p>
        <h1>{{ job?.title ?? 'Download not found' }}</h1>
      </div>
      <StatusChip :tone="job ? statusTone(job.status) : 'danger'">
        {{ job?.status ?? 'missing' }}
      </StatusChip>
    </div>

    <section
      v-if="job"
      class="sectionBlock"
    >
      <div class="progressLine">
        <ProgressBar
          :value="job.progress"
          :label="`${job.title} progress`"
        />
        <strong>{{ job.progress }}%</strong>
      </div>
      <dl class="detailList">
        <div>
          <dt>File</dt>
          <dd>{{ job.outputName }}</dd>
        </div>
        <div>
          <dt>Quality</dt>
          <dd>{{ job.quality }}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{{ job.pageUrl }}</dd>
        </div>
        <div v-if="job.outputUri">
          <dt>Published URI</dt>
          <dd>{{ job.outputUri }}</dd>
        </div>
        <div v-if="job.outputPath">
          <dt>Private path</dt>
          <dd>{{ job.outputPath }}</dd>
        </div>
        <div v-if="job.outputBytes">
          <dt>Size</dt>
          <dd>{{ formatBytes(job.outputBytes) }}</dd>
        </div>
        <div v-if="job.errorMessage">
          <dt>Error</dt>
          <dd>{{ job.errorMessage }}</dd>
        </div>
      </dl>
      <div class="actionRow">
        <AppButton
          icon="arrow-left"
          to="/downloads"
        >
          Back
        </AppButton>
        <AppButton
          v-if="job.outputUri"
          icon="play"
          variant="primary"
          @click="openFile"
        >
          Open
        </AppButton>
        <AppButton
          v-if="job.status === 'failed'"
          icon="download"
          :disabled="isBusy"
          @click="retryDownload"
        >
          Retry
        </AppButton>
        <AppButton
          icon="activity"
          :disabled="isBusy"
          @click="deleteHistory"
        >
          Delete video
        </AppButton>
      </div>
      <p
        v-if="actionError"
        class="muted"
      >
        {{ actionError }}
      </p>
    </section>

    <section
      v-else
      class="emptyState"
    >
      <p>No matching download record exists.</p>
      <AppButton
        icon="arrow-left"
        to="/downloads"
      >
        Downloads
      </AppButton>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { deleteDownloadHistory, openDownload, startDownload } from '@/api/downloads';
import AppButton from '@/app/components/AppButton.vue';
import ProgressBar from '@/app/components/ProgressBar.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { useDownloadsStore, type DownloadJobStatus } from '@/stores/downloads';

const props = defineProps<{
  jobId: string;
}>();

const downloads = useDownloadsStore();
const router = useRouter();
const job = computed(() => downloads.findJob(props.jobId));
const isBusy = ref(false);
const actionError = ref<string | null>(null);

onMounted(async () => {
  await downloads.loadHistory();
});

function statusTone(status: DownloadJobStatus): 'default' | 'success' | 'warning' | 'danger' {
  if (status === 'done') {
    return 'success';
  }

  if (status === 'failed') {
    return 'danger';
  }

  if (status === 'cancelled') {
    return 'warning';
  }

  return 'default';
}

function formatBytes(value: number): string {
  if (value < 1024 * 1024) {
    return `${Math.round(value / 1024)} KB`;
  }

  return `${Math.round(value / (1024 * 1024))} MB`;
}

async function openFile() {
  const contentUri = job.value?.outputUri;
  if (!contentUri) {
    return;
  }

  await runAction(() => openDownload(contentUri));
}

async function retryDownload() {
  if (!job.value) {
    return;
  }

  const retryJob = job.value;
  await runAction(async () => {
    await startDownload({
      masterUrl: retryJob.masterUrl,
      mediaPlaylistUrl: retryJob.mediaPlaylistUrl,
      referer: retryJob.referer,
      userAgent: retryJob.userAgent,
      cookies: retryJob.cookies,
      outputName: retryJob.outputName,
      title: retryJob.title,
      pageUrl: retryJob.pageUrl,
      qualityLabel: retryJob.quality,
    });
    await downloads.loadHistory();
    await router.push('/downloads');
  });
}

async function deleteHistory() {
  await runAction(async () => {
    const jobs = await deleteDownloadHistory(props.jobId);
    downloads.replaceRecords(jobs);
    await router.push('/downloads');
  });
}

async function runAction(action: () => Promise<void>) {
  isBusy.value = true;
  actionError.value = null;
  try {
    await action();
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isBusy.value = false;
  }
}
</script>
