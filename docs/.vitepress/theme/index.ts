import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import Layout from "./Layout.vue";
import SpringBenchmarkChart from "./components/SpringBenchmarkChart.vue";

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    app.component("SpringBenchmarkChart", SpringBenchmarkChart);
  },
} satisfies Theme;
