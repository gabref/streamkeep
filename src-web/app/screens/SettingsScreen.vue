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
          <dd>{{ settings?.defaultOutputDirectory ?? 'Loading' }}</dd>
        </div>
        <div>
          <dt>Current destination</dt>
          <dd>{{ settings?.outputDirectory ?? 'Loading' }}</dd>
        </div>
      </div>
      <p
        v-if="errorMessage"
        class="inlineError"
      >
        {{ errorMessage }}
      </p>
    </section>

    <section class="toolBand">
      <AppButton
        icon="download"
        :disabled="isBusy"
        @click="chooseFolder"
      >
        Choose folder
      </AppButton>
      <AppButton
        icon="sliders"
        :disabled="isBusy || !settings?.customOutputDirectory"
        @click="resetFolder"
      >
        Reset folder
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
import { open } from '@tauri-apps/plugin-dialog';
import { onMounted, ref } from 'vue';
import {
  getDownloadSettings,
  resetDownloadDirectory,
  setDownloadDirectory,
  type DownloadSettings,
} from '@/api/settings';
import AppButton from '@/app/components/AppButton.vue';
import StatusChip from '@/app/components/StatusChip.vue';

const settings = ref<DownloadSettings | null>(null);
const isBusy = ref(false);
const errorMessage = ref<string | null>(null);

onMounted(async () => {
  await runAction(async () => {
    settings.value = await getDownloadSettings();
  });
});

async function chooseFolder() {
  await runAction(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Choose Streamkeep folder',
    });
    if (typeof selected !== 'string') {
      return;
    }
    settings.value = await setDownloadDirectory(selected);
  });
}

async function resetFolder() {
  await runAction(async () => {
    settings.value = await resetDownloadDirectory();
  });
}

async function runAction(action: () => Promise<void>) {
  isBusy.value = true;
  errorMessage.value = null;
  try {
    await action();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isBusy.value = false;
  }
}
</script>
