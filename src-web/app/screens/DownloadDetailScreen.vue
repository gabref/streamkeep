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
      <ProgressBar
        :value="job.progress"
        :label="`${job.title} progress`"
      />
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
        <div v-if="job.outputPath">
          <dt>Output</dt>
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
          v-if="job.status === 'failed'"
          icon="download"
          variant="primary"
        >
          Retry
        </AppButton>
      </div>
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
import { computed, onMounted } from 'vue';
import AppButton from '@/app/components/AppButton.vue';
import ProgressBar from '@/app/components/ProgressBar.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { useDownloadsStore, type DownloadJobStatus } from '@/stores/downloads';

const props = defineProps<{
  jobId: string;
}>();

const downloads = useDownloadsStore();
const job = computed(() => downloads.findJob(props.jobId));

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
</script>
