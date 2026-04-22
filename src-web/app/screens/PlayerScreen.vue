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
      <AppButton icon="arrow-left">
        Back
      </AppButton>
      <AppButton icon="activity">
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
      <div class="browserFrame__bar">
        <span>https://</span>
        <input
          aria-label="Player URL"
          class="field"
          inputmode="url"
          value=""
        >
      </div>
      <div class="browserFrame__body">
        <IconGlyph name="shield" />
        <p>Embedded browsing will be connected in the next milestone.</p>
      </div>
    </section>

    <section class="sectionBlock">
      <h2>Detection state</h2>
      <div class="metricGrid">
        <div class="metricTile">
          <span>Status</span>
          <strong>Idle</strong>
        </div>
        <div class="metricTile">
          <span>Last request</span>
          <strong>None</strong>
        </div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import AppButton from '@/app/components/AppButton.vue';
import IconGlyph from '@/app/components/IconGlyph.vue';
import StatusChip from '@/app/components/StatusChip.vue';
</script>
