import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import SpringBenchmarkChart from "./components/SpringBenchmarkChart.vue";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("SpringBenchmarkChart", SpringBenchmarkChart);
  },
} satisfies Theme;
