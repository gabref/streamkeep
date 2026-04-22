<template>
  <main class="homeScreen">
    <section
      class="homeScreen__summary"
      aria-labelledby="home-title"
    >
      <p class="homeScreen__eyebrow">
        Android capture workspace
      </p>
      <h1 id="home-title">
        Streamkeep
      </h1>
      <p class="homeScreen__body">
        Ready for the Android bootstrapping pass. Stream detection and downloads will be added in
        the planned native and Rust stages.
      </p>
      <div class="homeScreen__actions">
        <button
          class="button button--primary"
          type="button"
          disabled
        >
          Open Player
        </button>
        <button
          class="button"
          type="button"
          disabled
        >
          Downloads
        </button>
      </div>
    </section>

    <section
      class="statusPanel"
      aria-labelledby="status-title"
    >
      <div>
        <p class="statusPanel__label">
          Status
        </p>
        <h2 id="status-title">
          Bootstrap build
        </h2>
      </div>
      <dl class="statusPanel__list">
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
  </main>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { getHealth, type HealthSnapshot } from '@/api/commands';

const health = ref<HealthSnapshot | null>(null);

onMounted(async () => {
  health.value = await getHealth();
});
</script>

