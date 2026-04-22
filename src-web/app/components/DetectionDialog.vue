<template>
  <Teleport to="body">
    <div
      class="modalBackdrop"
      role="presentation"
    >
      <section
        aria-labelledby="detection-dialog-title"
        aria-modal="true"
        class="confirmSheet"
        role="dialog"
      >
        <header class="confirmSheet__header">
          <div>
            <p class="eyebrow">
              Video detected
            </p>
            <h2 id="detection-dialog-title">
              Download this video?
            </h2>
          </div>
          <StatusChip tone="success">
            Stream detected
          </StatusChip>
        </header>

        <div class="fieldGroup">
          <span>Suggested title</span>
          <strong class="suggestedTitle">{{ stream.titleSuggestion }}</strong>
        </div>

        <label class="fieldGroup">
          <span>File name</span>
          <span class="fileNameControl">
            <input
              v-model="fileNameStem"
              aria-label="File name"
              class="field"
              autocomplete="off"
              autocapitalize="sentences"
              spellcheck="false"
              type="text"
            >
            <span class="fileNameControl__suffix">.mp4</span>
          </span>
        </label>

        <label class="fieldGroup">
          <span>Quality</span>
          <select
            v-model="selectedQuality"
            class="field"
          >
            <option
              v-for="quality in stream.qualities"
              :key="quality.id"
              :value="quality.id"
            >
              {{ quality.label }}
            </option>
          </select>
        </label>

        <dl class="detailList">
          <div>
            <dt>Output</dt>
            <dd>{{ finalFileName }}</dd>
          </div>
          <div>
            <dt>Source</dt>
            <dd>{{ stream.pageUrl || stream.masterUrl }}</dd>
          </div>
        </dl>

        <footer class="actionRow actionRow--end">
          <AppButton @click="$emit('cancel')">
            Cancel
          </AppButton>
          <AppButton
            icon="download"
            variant="primary"
            @click="emitDownload"
          >
            Download
          </AppButton>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import AppButton from '@/app/components/AppButton.vue';
import StatusChip from '@/app/components/StatusChip.vue';
import { buildMp4FileName, getInitialFileStem, type DetectedStream } from '@/domain/detection';

const props = defineProps<{
  stream: DetectedStream;
}>();

const emit = defineEmits<{
  cancel: [];
  download: [fileNameStem: string, qualityId: string];
}>();

const fileNameStem = ref(getInitialFileStem(props.stream));
const selectedQuality = ref(props.stream.qualities[0]?.id ?? 'best-available');

const finalFileName = computed(() => buildMp4FileName(fileNameStem.value));

watch(
  () => props.stream,
  (stream) => {
    fileNameStem.value = getInitialFileStem(stream);
    selectedQuality.value = stream.qualities[0]?.id ?? 'best-available';
  }
);

function emitDownload() {
  blurActiveElement();
  emit('download', fileNameStem.value, selectedQuality.value);
}

function blurActiveElement() {
  const activeElement = globalThis.document?.activeElement;
  if (activeElement instanceof globalThis.HTMLElement) {
    activeElement.blur();
  }
}
</script>
