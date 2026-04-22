<template>
  <RouterLink
    v-if="to"
    class="appButton"
    :class="buttonClass"
    :to="to"
  >
    <span
      v-if="icon"
      class="appButton__icon"
      aria-hidden="true"
    >
      <IconGlyph :name="icon" />
    </span>
    <span><slot /></span>
  </RouterLink>
  <button
    v-else
    class="appButton"
    :class="buttonClass"
    :type="type"
    :disabled="disabled"
  >
    <span
      v-if="icon"
      class="appButton__icon"
      aria-hidden="true"
    >
      <IconGlyph :name="icon" />
    </span>
    <span><slot /></span>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { RouterLink } from 'vue-router';
import IconGlyph, { type IconName } from '@/app/components/IconGlyph.vue';

const props = withDefaults(
  defineProps<{
    disabled?: boolean;
    icon?: IconName;
    to?: string;
    type?: 'button' | 'submit' | 'reset';
    variant?: 'primary' | 'secondary' | 'danger';
  }>(),
  {
    disabled: false,
    icon: undefined,
    to: undefined,
    type: 'button',
    variant: 'secondary',
  }
);

const buttonClass = computed(() => {
  return {
    'appButton--primary': props.variant === 'primary',
    'appButton--danger': props.variant === 'danger',
  };
});
</script>

