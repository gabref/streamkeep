<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Android capture workspace
        </p>
        <h1>Streamkeep</h1>
      </div>
      <StatusChip tone="success">
        Ready
      </StatusChip>
    </div>

    <section class="heroPanel">
      <div>
        <p class="heroPanel__copy">
          Open the player, sign in manually, start playback, and Streamkeep will listen for the
          HLS master playlist.
        </p>
        <div class="actionRow">
          <AppButton
            icon="play"
            to="/player"
            variant="primary"
          >
            Open Player
          </AppButton>
          <AppButton
            icon="download"
            to="/downloads"
          >
            Downloads
          </AppButton>
        </div>
      </div>
      <dl class="healthGrid">
        <div>
          <dt>App</dt>
          <dd>{{ health?.appName ?? 'Streamkeep' }}</dd>
        </div>
        <div>
          <dt>Version</dt>
          <dd>{{ health?.appVersion ?? '0.1.0' }}</dd>
        </div>
        <div>
          <dt>Platform</dt>
          <dd>{{ health?.targetPlatform ?? 'web preview' }}</dd>
        </div>
      </dl>
    </section>

    <section class="sectionBlock">
      <div class="sectionTitleRow">
        <h2>Recent downloads</h2>
        <RouterLink to="/downloads">
          View all
        </RouterLink>
      </div>
      <div class="listStack">
        <RouterLink
          v-for="job in downloads.recentJobs"
          :key="job.id"
          class="jobRow"
          :to="`/downloads/${job.id}`"
        >
          <span>
            <strong>{{ job.title }}</strong>
            <small>{{ job.outputName }}</small>
          </span>
          <StatusChip :tone="statusTone(job.status)">
            {{ statusLabel(job.status) }}
          </StatusChip>
        </RouterLink>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import { getHealth, type HealthSnapshot } from '@/api/commands';
import AppButton from '@/app/components/AppButton.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { useDownloadsStore, type DownloadJobStatus } from '@/stores/downloads';

const health = ref<HealthSnapshot | null>(null);
const downloads = useDownloadsStore();

onMounted(async () => {
  const [healthSnapshot] = await Promise.all([getHealth(), downloads.loadHistory()]);
  health.value = healthSnapshot;
});

function statusLabel(status: DownloadJobStatus): string {
  return status.replace('-', ' ');
}

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
</script>
