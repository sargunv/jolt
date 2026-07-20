<script setup lang="ts">
import benchmark from "../../../../tools/bench/reports/machines/linux-x86-64-2b358ab7aaf1.json";

/**
 * Benchmark readout: one row per tool, bars on a linear scale against the
 * slowest tool. Jolt's bars are meant to look almost absent — that thinness
 * is the measurement, not a styling accident.
 */

const corpus = benchmark.corpora.realistic;

const rows = Object.entries(corpus.whole_cli.tools)
  .map(([id, tool]) => ({
    id,
    label: tool.label,
    seconds: tool.timing.summary.median_ns / 1_000_000_000,
  }))
  .sort((a, b) => a.seconds - b.seconds);

const slowest = rows[rows.length - 1].seconds;

const machine = benchmark.machine.processor.replace(/ w\/ .*/, "");
const fileCount = corpus.manifest.files.toLocaleString("en-US");
const megabytes = (corpus.manifest.source_bytes / 1_000_000).toFixed(1);
</script>

<template>
  <figure class="bench">
    <div class="bench-rows">
      <div
        v-for="row in rows"
        :key="row.id"
        class="bench-row"
        :class="{ 'bench-row--jolt': row.id.startsWith('jolt') }"
      >
        <span class="bench-label">{{ row.label }}</span>
        <span class="bench-track">
          <span
            class="bench-bar"
            :style="{ width: `${(row.seconds / slowest) * 100}%` }"
          />
        </span>
        <span class="bench-value">{{ row.seconds.toFixed(2) }}&hairsp;s</span>
      </div>
    </div>
    <figcaption class="bench-caption">
      Spring Framework sources, {{ fileCount }} files, {{ megabytes }}&hairsp;MB.
      Whole-CLI median of five runs on {{ machine }}. Linear scale; lower is
      better.
    </figcaption>
  </figure>
</template>

<style scoped>
.bench {
  margin: 22px 0;
  border: 1px solid var(--jz-line);
  background: var(--jz-panel);
  padding: 18px 20px 14px;
}

.bench-rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.bench-row {
  display: grid;
  grid-template-columns: 11.5rem 1fr 4.5rem;
  align-items: center;
  gap: 14px;
  font-family: var(--jz-font-mono);
}

.bench-label {
  font-size: 12px;
  color: var(--jz-ink-2);
  white-space: nowrap;
}

.bench-row--jolt .bench-label {
  color: var(--jz-ink);
  font-weight: 500;
}

.bench-track {
  position: relative;
  height: 12px;
  border-left: 1px solid var(--jz-line-strong);
}

.bench-bar {
  position: absolute;
  inset: 0 auto 0 0;
  min-width: 2px;
  background: var(--jz-line-strong);
}

.bench-row--jolt .bench-bar {
  background: var(--jz-amber);
}

.bench-value {
  font-size: 12px;
  text-align: right;
  color: var(--jz-ink-2);
  font-variant-numeric: tabular-nums;
}

.bench-row--jolt .bench-value {
  color: var(--jz-amber);
  font-weight: 600;
}

.bench-caption {
  margin-top: 16px;
  padding-top: 10px;
  border-top: 1px solid var(--jz-line);
  font-family: var(--jz-font-mono);
  font-size: 11px;
  line-height: 1.6;
  color: var(--jz-ink-3);
}

@media (max-width: 640px) {
  .bench-row {
    grid-template-columns: 7.5rem 1fr 4rem;
    gap: 10px;
  }

  .bench-label {
    font-size: 11px;
  }
}
</style>
