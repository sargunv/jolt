import { defineConfig } from "vitepress";

export default defineConfig({
  base: process.env.VITEPRESS_BASE ?? "/",
  title: "Jolt",
  description:
    "Fast, opinionated JVM and Kotlin Multiplatform project tooling.",
  vite: {
    optimizeDeps: {
      include: [
        "@codemirror/lang-java",
        "@codemirror/language",
        "@codemirror/lint",
        "@codemirror/theme-one-dark",
        "@dprint/formatter",
        "codemirror",
        "vue-codemirror6",
      ],
    },
  },
  themeConfig: {
    sidebar: [
      { text: "What is Jolt?", link: "/what-is-jolt" },
      { text: "Installation", link: "/installation" },
      { text: "Configuration", link: "/configuration" },
      { text: "dprint Plugin", link: "/dprint-plugin" },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/sargunv/jolt" }],
  },
});
