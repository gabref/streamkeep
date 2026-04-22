<template>
  <div class="mainShell">
    <header class="mainShell__header">
      <RouterLink
        class="mainShell__brand"
        to="/"
      >
        <span class="mainShell__mark">S</span>
        <span>Streamkeep</span>
      </RouterLink>
      <nav
        class="mainShell__topNav"
        aria-label="Primary"
      >
        <RouterLink to="/player">
          Player
        </RouterLink>
        <RouterLink to="/downloads">
          Downloads
        </RouterLink>
        <RouterLink to="/settings">
          Settings
        </RouterLink>
      </nav>
    </header>

    <main class="mainShell__content">
      <slot />
    </main>

    <nav
      class="mainShell__bottomNav"
      aria-label="Primary"
    >
      <RouterLink to="/">
        <IconGlyph name="home" />
        <span>Home</span>
      </RouterLink>
      <RouterLink to="/player">
        <IconGlyph name="play" />
        <span>Player</span>
      </RouterLink>
      <RouterLink to="/downloads">
        <IconGlyph name="download" />
        <span>Downloads</span>
      </RouterLink>
      <RouterLink to="/settings">
        <IconGlyph name="settings" />
        <span>Settings</span>
      </RouterLink>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router';
import IconGlyph from '@/app/components/IconGlyph.vue';
</script>

