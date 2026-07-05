import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Jolt",
  description:
    "Fast, opinionated JVM and Kotlin Multiplatform project tooling.",
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
