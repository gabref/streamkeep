<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Preferences
        </p>
        <h1>Settings</h1>
      </div>
      <StatusChip>Local</StatusChip>
    </div>

    <section class="sectionBlock">
      <label class="toggleRow">
        <span>
          <strong>Keep web session</strong>
          <small>Preserve cookies between launches.</small>
        </span>
        <input
          type="checkbox"
          checked
        >
      </label>
      <label class="toggleRow">
        <span>
          <strong>Debug logging</strong>
          <small>Keep request diagnostics visible while developing.</small>
        </span>
        <input type="checkbox">
      </label>
      <div class="detailList">
        <div>
          <dt>Default destination</dt>
          <dd>Downloads/Streamkeep</dd>
        </div>
      </div>
    </section>

    <section class="toolBand">
      <AppButton icon="sliders">
        Clear session data
      </AppButton>
      <AppButton
        icon="bug"
        to="/diagnostics"
      >
        Diagnostics
      </AppButton>
    </section>
  </section>
</template>

<script setup lang="ts">
import AppButton from '@/app/components/AppButton.vue';
import StatusChip from '@/app/components/StatusChip.vue';
</script>

