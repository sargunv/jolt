<script setup lang="ts">
import { defineAsyncComponent } from "vue";
import FitOrBreakHero from "./FitOrBreakHero.vue";
import BenchStrip from "./BenchStrip.vue";

const JoltPlayground = defineAsyncComponent(
  () => import("./JoltPlayground.vue"),
);

const MANUAL = [
  {
    name: "What is Jolt?",
    desc: "Approach, performance, and where the speed comes from.",
    href: "/guides/what-is-jolt",
  },
  {
    name: "Installation",
    desc: "Binaries, installers, checksums, attestation.",
    href: "/guides/installation",
  },
  {
    name: "Configuration",
    desc: "jolt.toml, discovery, and the three fields.",
    href: "/guides/configuration",
  },
  {
    name: "Integrations",
    desc: "dprint, editors, and pre-commit hooks.",
    href: "/guides/integrations",
  },
  {
    name: "CLI reference",
    desc: "Every command and flag, generated from the binary.",
    href: "/reference/cli",
  },
  {
    name: "How the formatter works",
    desc: "CST, trivia, and the document IR.",
    href: "/internals/formatter",
  },
];
</script>

<template>
  <article class="home">
    <header class="home-head">
      <p class="home-pipeline">
        source → parse → cst → document → render → text
      </p>
      <h1 class="home-title">Jolt<span class="jz-caret" /></h1>
      <p class="home-lede">
        Jolt is a formatter for Java and Kotlin, shipped as a static native
        CLI and a dprint plugin. This page runs the same engine as WebAssembly
        in your browser.
      </p>
      <p class="home-links">
        <a href="#playground">Playground ↓</a>
        <a href="https://github.com/sargunv/jolt" target="_blank" rel="noopener"
          >GitHub ↗</a
        >
      </p>
    </header>

    <section id="install" class="home-section">
      <h2 class="home-h2">Install</h2>
      <div class="term">
        <pre><span class="term-p">$ </span>curl --proto '=https' --tlsv1.2 -LsSf \
    https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.sh | sh
<span class="term-p">$ </span>jolt fmt .</pre>
      </div>
      <p class="home-more">
        <a href="/guides/installation"
          >eget, mise, dprint, Windows, manual download →</a
        >
      </p>
    </section>

    <section id="benchmark" class="home-section">
      <h2 class="home-h2">Benchmark</h2>
      <BenchStrip />
      <p class="home-more">
        <a href="/guides/what-is-jolt">More about the approach →</a>
      </p>
    </section>

    <section id="fit-or-break" class="home-section">
      <h2 class="home-h2">Fit or break</h2>
      <ClientOnly>
        <FitOrBreakHero />
      </ClientOnly>
    </section>

    <section id="playground" class="home-section home-section--wide">
      <div class="home-section-inner">
        <h2 class="home-h2">Playground</h2>
      </div>
      <ClientOnly>
        <JoltPlayground />
      </ClientOnly>
    </section>

    <section id="manual" class="home-section">
      <h2 class="home-h2">The manual</h2>
      <ol class="manual">
        <li v-for="entry in MANUAL" :key="entry.href" class="manual-item">
          <a :href="entry.href" class="manual-link">
            <span class="manual-name">{{ entry.name }}</span>
            <span class="manual-desc">{{ entry.desc }}</span>
          </a>
        </li>
      </ol>
    </section>
  </article>
</template>

<style scoped>
.home {
  padding: 56px 0 80px;
}

.home-head,
.home-section {
  width: min(100% - 48px, 688px);
  margin-inline: auto;
}

.home-section--wide {
  width: min(100% - 48px, 1120px);
}

.home-section-inner {
  max-width: 688px;
  margin-inline: auto;
}

/* Header ----------------------------------------------------------- */

.home-pipeline {
  font-family: var(--jz-font-mono);
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--jz-ink-3);
  margin: 0 0 22px;
  overflow-wrap: normal;
  white-space: nowrap;
  overflow-x: auto;
  scrollbar-width: none;
}

.home-pipeline::-webkit-scrollbar {
  display: none;
}

.home-title {
  font-family: var(--jz-font-mono);
  font-size: 60px;
  font-weight: 600;
  letter-spacing: 0.01em;
  line-height: 1.05;
  color: var(--jz-ink);
  margin: 0 0 22px;
}

.home-lede {
  font-size: 17px;
  line-height: 1.66;
  color: var(--jz-ink);
  margin: 0 0 20px;
  max-width: 56ch;
}

.home-links {
  display: flex;
  gap: 24px;
  margin: 0;
  font-family: var(--jz-font-mono);
  font-size: 13px;
}

.home-links a {
  color: var(--jz-amber);
  text-decoration: underline;
  text-decoration-color: var(--jz-line-strong);
  text-underline-offset: 3px;
}

.home-links a:hover {
  color: var(--jz-amber-strong);
  text-decoration-color: var(--jz-amber-strong);
}

/* Sections ---------------------------------------------------------- */

.home-section {
  margin-top: 64px;
}

.home-h2 {
  font-family: var(--jz-font-mono);
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--jz-ink);
  border-top: 1px solid var(--jz-line);
  padding-top: 30px;
  margin: 0 0 24px;
}

.home-section p {
  font-size: 15px;
  line-height: 1.72;
  color: var(--jz-ink);
  margin: 0 0 18px;
}

.home-more {
  font-family: var(--jz-font-mono);
  font-size: 12.5px !important;
}

.home-more a {
  color: var(--jz-amber);
  text-decoration: underline;
  text-decoration-color: var(--jz-line-strong);
  text-underline-offset: 3px;
}

.home-more a:hover {
  color: var(--jz-amber-strong);
  text-decoration-color: var(--jz-amber-strong);
}

/* Terminal ------------------------------------------------------------ */

.term {
  border: 1px solid var(--jz-line);
  background: var(--jz-panel);
  padding: 16px 20px;
  margin-bottom: 22px;
  overflow-x: auto;
}

.term pre {
  margin: 0;
  font-family: var(--jz-font-mono);
  font-size: 12.5px;
  line-height: 1.8;
  color: var(--jz-ink);
}

.term-p {
  color: var(--jz-amber);
  font-weight: 600;
  user-select: none;
}

/* Manual index -------------------------------------------------------- */

.manual {
  list-style: none;
  margin: 0;
  padding: 0;
  counter-reset: jz-man;
  border-top: 1px solid var(--jz-line);
}

.manual-item {
  counter-increment: jz-man;
  border-bottom: 1px solid var(--jz-line);
}

.manual-link {
  display: grid;
  grid-template-columns: 3.2rem 1fr;
  gap: 0 18px;
  padding: 13px 4px;
  text-decoration: none !important;
  align-items: baseline;
}

.manual-link::before {
  content: counter(jz-man, decimal-leading-zero);
  grid-row: span 2;
  font-family: var(--jz-font-mono);
  font-size: 12px;
  color: var(--jz-amber);
  padding-top: 2px;
}

.manual-name {
  font-family: var(--jz-font-mono);
  font-size: 14px;
  font-weight: 500;
  color: var(--jz-ink);
}

.manual-desc {
  font-size: 13.5px;
  color: var(--jz-ink-2);
  margin-top: 2px;
}

.manual-link:hover {
  background: var(--jz-panel);
}

.manual-link:hover .manual-name {
  color: var(--jz-amber);
}

/* Responsive ------------------------------------------------------------ */

@media (max-width: 640px) {
  .home {
    padding-top: 36px;
  }

  .home-title {
    font-size: 42px;
  }

  .home-lede {
    font-size: 16px;
  }

  .home-pipeline {
    font-size: 10px;
    letter-spacing: 0.07em;
  }
}
</style>
