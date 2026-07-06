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
    <template #home-hero-after>
      <ClientOnly>
        <JoltPlayground id="playground" />
      </ClientOnly>
    </template>
  </Layout>
</template>

<style>
.VPContent.is-home .VPHome {
  display: flex;
  flex-direction: column;
  min-height: calc(100dvh - var(--vp-nav-height, 64px));
  margin-bottom: 0 !important;
}

.Layout:has(.VPContent.is-home) .VPNavBarTitle {
  visibility: hidden;
}

.VPContent.is-home .VPHomeHero {
  flex-shrink: 0;
  padding: 64px 24px 24px;
}

.VPContent.is-home .VPHomeHero .main {
  max-width: 720px;
}

.VPContent.is-home .VPHomeHero .name,
.VPContent.is-home .VPHomeHero .text {
  max-width: 720px;
}

.VPContent.is-home .VPHomeHero .tagline {
  max-width: 620px;
}

.VPContent.is-home .VPHome > .jolt-playground {
  flex: 1 1 auto;
  width: min(100%, 1440px);
  min-height: 760px;
  margin: 0 auto;
  padding-top: 0;
}

@media (min-width: 960px) {
  .VPContent.is-home .VPHomeHero .container {
    align-items: flex-end;
  }
}

@media (max-width: 768px) {
  .VPContent.is-home .VPHomeHero {
    padding: 32px 24px 18px;
  }

  .VPContent.is-home .VPHome > .jolt-playground {
    min-height: 900px;
    padding-inline: 12px;
  }
}
</style>
