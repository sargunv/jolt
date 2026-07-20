<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

const props = withDefaults(
  defineProps<{ marker: number; min?: number; max?: number; scroll?: number }>(),
  { min: 24, max: 80, scroll: 0 },
);
const emit = defineEmits<{ "update:marker": [ch: number] }>();

const MAJORS = [10, 20, 30, 40, 50, 60, 70, 80];

const innerEl = ref<HTMLElement>();
const chPx = ref(7.8);

let observer: ResizeObserver | undefined;

function chFromClientX(clientX: number): number {
  const rect = innerEl.value?.getBoundingClientRect();
  if (!rect) return props.marker;
  const ch = Math.round((clientX - rect.left) / (rect.width / 80));
  return Math.max(props.min, Math.min(props.max, ch));
}

function onPointerDown(event: PointerEvent) {
  const ruler = event.currentTarget as HTMLElement;
  ruler.setPointerCapture(event.pointerId);
  emit("update:marker", chFromClientX(event.clientX));

  const onMove = (e: PointerEvent) => emit("update:marker", chFromClientX(e.clientX));
  const onUp = () => {
    ruler.removeEventListener("pointermove", onMove);
    ruler.removeEventListener("pointerup", onUp);
    ruler.removeEventListener("pointercancel", onUp);
  };
  ruler.addEventListener("pointermove", onMove);
  ruler.addEventListener("pointerup", onUp);
  ruler.addEventListener("pointercancel", onUp);
}

function onHandleKeydown(event: KeyboardEvent) {
  const step = event.shiftKey ? 10 : 1;
  if (event.key === "ArrowLeft") {
    emit("update:marker", Math.max(props.min, props.marker - step));
    event.preventDefault();
  } else if (event.key === "ArrowRight") {
    emit("update:marker", Math.min(props.max, props.marker + step));
    event.preventDefault();
  } else if (event.key === "Home") {
    emit("update:marker", props.min);
    event.preventDefault();
  } else if (event.key === "End") {
    emit("update:marker", props.max);
    event.preventDefault();
  }
}

onMounted(() => {
  const measure = () => {
    if (innerEl.value) {
      chPx.value = innerEl.value.getBoundingClientRect().width / 80;
    }
  };
  measure();
  observer = new ResizeObserver(measure);
  if (innerEl.value) observer.observe(innerEl.value);
});

onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <div ref="rulerEl" class="ruler" @pointerdown="onPointerDown">
    <div
      ref="innerEl"
      class="ruler-inner"
      :style="{ transform: `translateX(${-scroll}px)` }"
    >
      <div class="ruler-ticks" />
      <span
        v-for="n in MAJORS"
        :key="n"
        class="ruler-label"
        :class="{ 'ruler-label--last': n === 80 }"
        :style="{ left: `${n * chPx}px` }"
        >{{ n }}</span
      >
      <button
        type="button"
        class="ruler-handle"
        role="slider"
        aria-label="Line width"
        :aria-valuenow="marker"
        :aria-valuemin="min"
        :aria-valuemax="max"
        :style="{ left: `${marker * chPx}px` }"
        @keydown="onHandleKeydown"
      />
    </div>
  </div>
</template>

<style scoped>
.ruler {
  position: relative;
  width: 100%;
  height: 30px;
  font-family: var(--jz-font-mono);
  font-size: 13px;
  border-bottom: 1px solid var(--jz-line-strong);
  user-select: none;
  overflow: hidden;
  cursor: ew-resize;
  touch-action: none;
}

.ruler-inner {
  position: absolute;
  inset: 0 auto 0 0;
  width: 80ch;
}

/* Minor tick every 2ch, major every 10ch, rising from the rule line. */
.ruler-ticks {
  position: absolute;
  inset: auto 0 0 0;
  height: 5px;
  background-image: repeating-linear-gradient(
    90deg,
    var(--jz-line-strong) 0,
    var(--jz-line-strong) 1px,
    transparent 1px,
    transparent 2ch
  );
}

.ruler-ticks::after {
  content: "";
  position: absolute;
  inset: auto 0 0 0;
  height: 9px;
  background-image: repeating-linear-gradient(
    90deg,
    var(--jz-line-strong) 0,
    var(--jz-line-strong) 1px,
    transparent 1px,
    transparent 10ch
  );
}

.ruler-label {
  position: absolute;
  bottom: 11px;
  transform: translateX(-50%);
  font-size: 9px;
  letter-spacing: 0.04em;
  color: var(--jz-ink-3);
}

.ruler-label--last {
  transform: translateX(calc(-100% + 0.5ch));
}

.ruler-handle {
  position: absolute;
  bottom: -1px;
  width: 9px;
  height: 16px;
  padding: 0;
  transform: translateX(-50%);
  border: none;
  background: var(--jz-amber);
  cursor: ew-resize;
}

.ruler-handle::after {
  content: "";
  position: absolute;
  top: 4px;
  left: 50%;
  width: 3px;
  height: 8px;
  transform: translateX(-50%);
  background: var(--jz-hivis);
}

.ruler-handle:focus-visible {
  outline: 2px solid var(--jz-amber);
  outline-offset: 1px;
}
</style>
