<script setup lang="ts">
import { computed } from "vue";
import { Bar } from "vue-chartjs";
import {
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  LinearScale,
  Tooltip,
  type ChartData,
  type ChartOptions,
} from "chart.js";
import { useData } from "vitepress";
import benchmark from "../../../../tools/bench/reports/machines/linux-x86-64-2b358ab7aaf1.json";

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip);

const { isDark } = useData();
const tools = Object.entries(
  benchmark.corpora.realistic.whole_cli.tools,
).map(([id, tool]) => ({
  id,
  label: tool.label,
  medianSeconds: tool.timing.summary.median_ns / 1_000_000_000,
}));

const labels = tools.map((tool) => tool.label);
const seconds = tools.map((tool) => tool.medianSeconds);
const native = tools.find((tool) => tool.id === "jolt-native");
const googleJavaFormat = tools.find((tool) => tool.id === "google-java-format");
const speedup =
  native && googleJavaFormat
    ? googleJavaFormat.medianSeconds / native.medianSeconds
    : undefined;

function formatSeconds(value: number): string {
  return value < 1 ? value.toFixed(2) : value.toFixed(1);
}

function themeColor(cssVar: string, fallback: string): string {
  if (typeof document === "undefined") {
    return fallback;
  }

  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(cssVar)
    .trim();
  return value || fallback;
}

const chartData = computed<ChartData<"bar">>(() => {
  const jolt = themeColor("--vp-c-brand-1", "#3c8772");
  const joltSoft = themeColor("--vp-c-brand-soft", "#347062");
  const other = themeColor("--vp-c-text-3", "#9ca3af");

  return {
    labels,
    datasets: [
      {
        data: seconds,
        backgroundColor: [jolt, joltSoft, other, other],
        borderRadius: 4,
        barThickness: 22,
      },
    ],
  };
});

const chartOptions = computed<ChartOptions<"bar">>(() => ({
  indexAxis: "y",
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (context) => `${formatSeconds(context.parsed.x)}s`,
      },
    },
  },
  scales: {
    x: {
      min: 0,
      max: Math.ceil(Math.max(...seconds) / 5) * 5,
      title: {
        display: true,
        text: "Seconds",
        color: isDark.value ? "#e5e7eb" : "#374151",
      },
      ticks: {
        color: isDark.value ? "#9ca3af" : "#6b7280",
        callback: (value) => `${value}`,
      },
      grid: {
        color: isDark.value ? "#374151" : "#e5e7eb",
      },
    },
    y: {
      ticks: {
        color: isDark.value ? "#e5e7eb" : "#374151",
      },
      grid: {
        display: false,
      },
    },
  },
}));
</script>

<template>
  <div class="spring-benchmark-chart">
    <Bar :data="chartData" :options="chartOptions" />
  </div>
  <p>
    Measured on {{ benchmark.machine.processor }}. Median of five whole-CLI
    runs; lower is better.
  </p>
  <p v-if="native && speedup">
    In native mode, Jolt formats the full corpus in
    {{ formatSeconds(native.medianSeconds) }} seconds—{{ speedup.toFixed(1) }}×
    faster than <code>google-java-format</code> in the same run.
  </p>
</template>

<style scoped>
.spring-benchmark-chart {
  height: 220px;
  margin: 1.5rem 0;
  padding: 1rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
}
</style>
