<script setup lang="ts">
withDefaults(
  defineProps<{
    variant: "code" | "terminal";
    title?: string;
  }>(),
  { title: "" },
);
</script>

<template>
  <figure class="code-frame" :class="`code-frame--${variant}`">
    <figcaption class="code-frame-header">
      <slot v-if="variant === 'code'" name="header" />
      <template v-else>
        <span v-if="title" class="code-frame-title">{{ title }}</span>
        <span v-else class="code-frame-sr-only">Terminal window</span>
      </template>
    </figcaption>
    <div class="code-frame-body">
      <slot />
    </div>
  </figure>
</template>
