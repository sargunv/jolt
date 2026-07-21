<script setup lang="ts">
import { defineAsyncComponent } from "vue";
import { useData } from "vitepress";
import DefaultTheme from "vitepress/theme";

const HomePage = defineAsyncComponent(
  () => import("./components/HomePage.vue"),
);

const { frontmatter } = useData();
const { Layout } = DefaultTheme;
</script>

<template>
  <Layout v-if="frontmatter.layout === 'home'">
    <template #home-hero-before>
      <HomePage />
    </template>
  </Layout>
  <Layout v-else />
</template>

<style>
.VPContent.is-home .VPHome {
  margin-bottom: 0;
  padding: 0;
}
</style>
