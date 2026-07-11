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

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip);

const { isDark } = useData();

const labels = [
  "jolt (native)",
  "jolt (dprint)",
  "google-java-format",
  "prettier-java",
] as const;

const seconds = [0.30, 0.42, 11, 28];

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
    labels: [...labels],
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
        label: (context) => `${context.parsed.x}s`,
      },
    },
  },
  scales: {
    x: {
      min: 0,
      max: 30,
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
