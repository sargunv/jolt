<script setup lang="ts">
import { defineAsyncComponent } from "vue";
import DefaultTheme from "vitepress/theme";

const JoltPlayground = defineAsyncComponent(
  () => import("./components/JoltPlayground.vue"),
);

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #home-features-after>
      <ClientOnly>
        <JoltPlayground />
      </ClientOnly>
    </template>
  </Layout>
</template>

<style>
.VPContent.is-home {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: calc(100dvh - var(--vp-nav-height, 64px));
}

.VPContent.is-home .VPHome {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  margin-bottom: 0 !important;
}

/* Empty markdown wrapper after the playground slot. */
.VPContent.is-home .VPHome > div[style*="position"] {
  display: none;
}
</style>
