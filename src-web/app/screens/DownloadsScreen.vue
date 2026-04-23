<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Local history
        </p>
        <h1>Downloads</h1>
      </div>
      <StatusChip>{{ downloads.jobs.length }} jobs</StatusChip>
    </div>

    <section
      v-if="downloads.activeJobs.length"
      class="sectionBlock"
    >
      <h2>Active</h2>
      <div class="listStack">
        <RouterLink
          v-for="job in downloads.activeJobs"
          :key="job.id"
          class="jobCard"
          :to="`/downloads/${job.id}`"
        >
          <span class="jobCard__title">{{ job.title }}</span>
          <span class="muted">{{ job.outputName }}</span>
          <div class="progressLine">
            <ProgressBar
              :value="job.progress"
              :label="`${job.title} progress`"
            />
            <strong>{{ job.progress }}%</strong>
          </div>
        </RouterLink>
      </div>
    </section>

    <section class="sectionBlock">
      <h2>Completed</h2>
      <p
        v-if="!downloads.completedJobs.length"
        class="muted"
      >
        No completed downloads yet.
      </p>
      <div
        v-else
        class="listStack"
      >
        <RouterLink
          v-for="job in downloads.completedJobs"
          :key="job.id"
          class="jobRow"
          :to="`/downloads/${job.id}`"
        >
          <span>
            <strong>{{ job.title }}</strong>
            <small>{{ job.outputName }}</small>
          </span>
          <StatusChip tone="success">
            Done
          </StatusChip>
        </RouterLink>
      </div>
    </section>

    <section
      v-if="downloads.failedJobs.length"
      class="sectionBlock"
    >
      <h2>Failed</h2>
      <div class="listStack">
        <RouterLink
          v-for="job in downloads.failedJobs"
          :key="job.id"
          class="jobRow"
          :to="`/downloads/${job.id}`"
        >
          <span>
            <strong>{{ job.title }}</strong>
            <small>{{ job.errorMessage }}</small>
          </span>
          <StatusChip tone="danger">
            Failed
          </StatusChip>
        </RouterLink>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { RouterLink } from 'vue-router';
import ProgressBar from '@/app/components/ProgressBar.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { useDownloadsStore } from '@/stores/downloads';
import { clearActiveTextInteraction } from '@/utils/dom';

const downloads = useDownloadsStore();

onMounted(async () => {
  clearActiveTextInteraction();
  await downloads.loadHistory();
});
</script>
