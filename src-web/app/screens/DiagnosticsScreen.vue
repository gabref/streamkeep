<template>
  <section class="screen">
    <div class="screenHeader">
      <div>
        <p class="eyebrow">
          Diagnostics
        </p>
        <h1>Debug</h1>
      </div>
      <StatusChip>Quiet</StatusChip>
    </div>

    <section class="sectionBlock">
      <h2>Capture events</h2>
      <div class="emptyState">
        <p>No intercepted requests have been reported yet.</p>
      </div>
    </section>

    <section class="sectionBlock">
      <h2>Latest stream payload</h2>
      <pre class="codeBlock">{}</pre>
    </section>
  </section>
</template>

<script setup lang="ts">
import StatusChip from '@/app/components/StatusChip.vue';
</script>

