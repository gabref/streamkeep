<template>
  <MainShell>
    <RouterView v-slot="{ Component, route }">
      <Transition
        mode="out-in"
        name="screenSwap"
      >
        <component
          :is="Component"
          :key="route.fullPath"
        />
      </Transition>
    </RouterView>
  </MainShell>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router';
import MainShell from '@/app/layouts/MainShell.vue';
</script>

