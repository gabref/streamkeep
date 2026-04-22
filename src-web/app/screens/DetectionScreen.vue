<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Detection dialog
        </p>
        <h1>Video detected</h1>
      </div>
      <StatusChip tone="warning">
        Awaiting confirmation
      </StatusChip>
    </div>

    <section class="dialogPreview">
      <div class="dialogPreview__header">
        <h2>Download this video?</h2>
        <StatusChip>master.m3u8</StatusChip>
      </div>
      <label class="fieldGroup">
        <span>Suggested title</span>
        <input
          class="field"
          value="Detected stream"
        >
      </label>
      <label class="fieldGroup">
        <span>File name</span>
        <input
          class="field"
          value="Detected stream.mp4"
        >
      </label>
      <label class="fieldGroup">
        <span>Quality</span>
        <select class="field">
          <option>Best available</option>
          <option>1080p</option>
          <option>720p</option>
        </select>
      </label>
      <div class="actionRow actionRow--end">
        <AppButton>Cancel</AppButton>
        <AppButton
          icon="download"
          variant="primary"
        >
          Download as MP4
        </AppButton>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import AppButton from '@/app/components/AppButton.vue';
import StatusChip from '@/app/components/StatusChip.vue';
</script>

