import { defineConfig } from "vitepress";

/**
 * Shiki theme whose colors are the site's own CSS variables, so code blocks
 * follow the datasheet (light) and instrument (dark) palettes with a single
 * registration.
 */
const joltShikiTheme = {
  name: "jolt",
  fg: "var(--jz-ink)",
  bg: "var(--jz-panel)",
  settings: [
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: { foreground: "var(--jz-ink-3)", fontStyle: "italic" },
    },
    {
      scope: ["keyword", "storage", "punctuation.definition.keyword"],
      settings: { foreground: "var(--jz-steel)" },
    },
    {
      scope: ["string", "constant.character", "markup.quote"],
      settings: { foreground: "var(--jz-string)" },
    },
    {
      // Shell grammars mark bare words as unquoted strings; keep those ink
      // so commands read as commands, not as one long string literal.
      scope: ["string.unquoted"],
      settings: { foreground: "var(--jz-ink)" },
    },
    {
      scope: ["constant.numeric", "constant.language", "variable.language"],
      settings: { foreground: "var(--jz-teal)" },
    },
    {
      scope: ["variable", "entity.name.variable", "support.variable"],
      settings: { foreground: "var(--jz-ink)" },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
        "entity.name.function",
        "support.function",
      ],
      settings: { foreground: "var(--jz-ink)" },
    },
    {
      scope: ["keyword.operator", "punctuation", "meta.brace"],
      settings: { foreground: "var(--jz-ink-2)" },
    },
    {
      scope: ["meta.annotation", "storage.type.annotation"],
      settings: { foreground: "var(--jz-ink-2)" },
    },
  ],
};

export default defineConfig({
  base: process.env.VITEPRESS_BASE ?? "/",
  title: "Jolt",
  description:
    "Jolt is a fast, opinionated formatter for Java and Kotlin. One static binary, no JVM required.",
  vite: {
    optimizeDeps: {
      include: [
        "@codemirror/lang-java",
        "@codemirror/language",
        "@codemirror/lint",
        "@dprint/formatter",
        "codemirror",
        "vue-codemirror6",
      ],
    },
  },
  markdown: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    theme: joltShikiTheme as any,
  },
  themeConfig: {
    nav: [
      { text: "Guides", link: "/guides/what-is-jolt" },
      { text: "Reference", link: "/reference/cli" },
      { text: "Internals", link: "/internals/formatter" },
    ],
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
    outline: {
      label: "On this page",
    },
    search: {
      provider: "local",
    },
    socialLinks: [{ icon: "github", link: "https://github.com/sargunv/jolt" }],
    footer: {
      message:
        '\u003ca href="https://github.com/sargunv/jolt" target="_blank" rel="noopener"\u003egithub.com/sargunv/jolt\u003c/a\u003e',
    },
  },
});
