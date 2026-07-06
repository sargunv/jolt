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
      {
        text: "Guides",
        items: [
          { text: "What is Jolt?", link: "/guides/what-is-jolt" },
          { text: "Installation", link: "/guides/installation" },
          { text: "Configuration", link: "/guides/configuration" },
          { text: "Integrations", link: "/guides/integrations" },
        ],
      },
      {
        text: "Reference",
        items: [{ text: "CLI", link: "/reference/cli" }],
      },
      {
        text: "Internals",
        items: [{ text: "Formatter", link: "/internals/formatter" }],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/sargunv/jolt" }],
  },
});
