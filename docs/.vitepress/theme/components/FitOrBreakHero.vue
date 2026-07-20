<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { LanguageSupport } from "@codemirror/language";
import { java } from "@codemirror/lang-java";
import SourceEditor from "./SourceEditor.vue";
import ColumnRuler from "./ColumnRuler.vue";
import { kotlinLanguage } from "../kotlinLanguage";
import { useJoltFormatter } from "../composables/useJoltFormatter";

/**
 * The fit-or-break instrument. Two scrollable sample files, formatted live
 * by the real WASM engine, at a line width the reader sets by dragging the
 * column edge.
 */

const WORKSPACE_KT = `package dev.jolt.bench

import dev.jolt.core.project
import dev.jolt.core.Version
import dev.jolt.core.Target

data class Artifact(val group: String, val name: String, val version: Version, val targets: Set<Target>)

class Registry<T : Any> {
  private val entries = mutableMapOf<String, T>()

  fun register(name: String, entry: T): T? {
    require(name.isNotBlank()) { "blank name" }
    return entries.put(name, entry)
  }

  inline fun <reified R : T> find(name: String, fallback: R? = null): R? {
    return entries[name] as? R ?: fallback
  }
}

fun <K, V> Map<K, V>.invert(): Map<V, K> = entries.associate { (key, value) -> value to key }

val workspace = project(group = "dev.sargunv", name = "jolt") {
  targets {
    jvm { toolchain = 21 }
    androidLibrary { minSdk = 26; compileSdk = 35 }
  }
  dependencies {
    implementation("com.squareup.okio:okio:3.9.0")
    testImplementation(kotlin("test"))
  }
}

fun describe(a: Artifact) = a.targets.sortedBy { it.ordinal }.joinToString(separator = "|", prefix = "\${a.group}:") { it.name.lowercase() }
`;

const RESOLVER_JAVA = `import java.util.Optional;
import java.util.HashMap;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.function.Predicate;
import java.util.stream.Collectors;

public final class Resolver {
  private final Map<Coordinates, Artifact> cache = new HashMap<>();

  // @formatter:off
  static final int RETRIES  = 3;
  static final int TIMEOUT  = 5000;
  static final int CACHE_MB = 64;
  // @formatter:on

  public <T extends Artifact> Optional<T> resolve(Coordinates coordinates, Class<T> type, Predicate<? super T> filter) {
    return Optional.ofNullable(cache.computeIfAbsent(coordinates, key -> fetch(key, type)))
        .filter(type::isInstance)
        .map(type::cast)
        .filter(filter);
  }

  static <T extends Comparable<? super T>> List<T> topK(Collection<? extends T> values, int k) {
    return values.stream().sorted(Comparator.reverseOrder()).limit(k).collect(Collectors.toUnmodifiableList());
  }

  String report(Map<String, List<Artifact>> grouped, Locale locale) {
    return grouped.entrySet().stream()
        .map(entry -> String.format(locale, "%s=%d", entry.getKey(), entry.getValue().size()))
        .collect(Collectors.joining(", ", "{", "}"));
  }
}
`;

const kotlin = new LanguageSupport(kotlinLanguage);

const SAMPLES = [
  { label: "WORKSPACE.KT", path: "Example.kt", language: kotlin, source: WORKSPACE_KT },
  { label: "RESOLVER.JAVA", path: "Resolver.java", language: java(), source: RESOLVER_JAVA },
];

const MIN_CH = 24;
const MAX_CH = 80;

const { loading, loadError, ensureReady, formatSource } = useJoltFormatter();

const activeSample = ref(0);
const widthCh = ref(MAX_CH);
const output = ref(SAMPLES[0].source);
const availableCh = ref(MAX_CH);
const rulerPad = ref(19);
const rulerScroll = ref(0);
const rootEl = ref<HTMLElement>();
const probeEl = ref<HTMLElement>();
const chPx = ref(7.8);

let userDragged = false;
let generation = 0;
let rafPending = 0;
let observer: ResizeObserver | undefined;
let scrollerEl: Element | null = null;

const sample = computed(() => SAMPLES[activeSample.value]);

function measure() {
  if (probeEl.value) {
    chPx.value = probeEl.value.getBoundingClientRect().width / 80;
  }
  const content = rootEl.value?.querySelector(".cm-content");
  if (content && rootEl.value) {
    rulerPad.value =
      content.getBoundingClientRect().left -
      rootEl.value.getBoundingClientRect().left;
    availableCh.value = Math.max(
      MIN_CH,
      Math.min(
        MAX_CH,
        Math.floor((rootEl.value.clientWidth - rulerPad.value) / chPx.value),
      ),
    );
  }
  if (!userDragged || widthCh.value > availableCh.value) {
    widthCh.value = availableCh.value;
  }
}

function onScrollerScroll() {
  rulerScroll.value = scrollerEl?.scrollLeft ?? 0;
}

function attachScroller() {
  scrollerEl?.removeEventListener("scroll", onScrollerScroll);
  scrollerEl = rootEl.value?.querySelector(".cm-scroller") ?? null;
  scrollerEl?.addEventListener("scroll", onScrollerScroll, { passive: true });
  rulerScroll.value = scrollerEl?.scrollLeft ?? 0;
}

function runFormat() {
  const gen = ++generation;
  const current = sample.value;
  void formatSource(current.path, current.source, {
    lineWidth: widthCh.value,
    indentWidth: 2,
    useTabs: false,
  })
    .then((result) => {
      if (gen === generation) output.value = result;
    })
    .catch(() => {
      if (gen === generation) output.value = current.source;
    });
}

function selectSample(index: number) {
  if (index === activeSample.value) return;
  activeSample.value = index;
  output.value = sample.value.source;
  requestAnimationFrame(() => {
    attachScroller();
    if (scrollerEl) {
      scrollerEl.scrollTop = 0;
      scrollerEl.scrollLeft = 0;
    }
    measure();
  });
  if (!loading.value && !loadError.value) runFormat();
}

function onMarkerUpdate(ch: number) {
  userDragged = true;
  widthCh.value = ch;
}

watch(widthCh, () => {
  if (loading.value || loadError.value || rafPending) return;
  rafPending = requestAnimationFrame(() => {
    rafPending = 0;
    runFormat();
  });
});

onMounted(async () => {
  await document.fonts.ready;
  attachScroller();
  measure();
  observer = new ResizeObserver(measure);
  if (rootEl.value) observer.observe(rootEl.value);
  try {
    await ensureReady();
    runFormat();
  } catch {
    // loadError surfaces in the header
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
  scrollerEl?.removeEventListener("scroll", onScrollerScroll);
  if (rafPending) cancelAnimationFrame(rafPending);
});
</script>

<template>
  <div ref="rootEl" class="instrument">
    <div class="instrument-head">
      <div class="instrument-samples" role="group" aria-label="Sample file">
        <button
          v-for="(s, i) in SAMPLES"
          :key="s.path"
          type="button"
          class="instrument-sample"
          :class="{ 'instrument-sample--active': i === activeSample }"
          :aria-pressed="i === activeSample"
          @click="selectSample(i)"
        >
          {{ s.label }}
        </button>
      </div>
      <span class="instrument-readout">
        <template v-if="loadError">ENGINE FAILED TO LOAD</template>
        <template v-else-if="loading">LOADING ENGINE&hellip;</template>
        <template v-else
          >LINE-WIDTH&nbsp;=&nbsp;<b>{{ widthCh }}</b></template
        >
      </span>
    </div>

    <div class="instrument-scale" :style="{ paddingLeft: `${rulerPad}px` }">
      <ColumnRuler
        :marker="widthCh"
        :min="MIN_CH"
        :max="availableCh"
        :scroll="rulerScroll"
        @update:marker="onMarkerUpdate"
      />
    </div>

    <div class="instrument-pane">
      <SourceEditor
        v-if="!loadError"
        :key="sample.path"
        :model-value="output"
        :language="sample.language"
        :line-width="widthCh"
        show-ruler
        readonly
      />
      <div v-else class="instrument-error">{{ loadError }}</div>
    </div>

    <span ref="probeEl" class="instrument-probe" aria-hidden="true">{{
      "0".repeat(80)
    }}</span>
  </div>
</template>

<style scoped>
.instrument {
  position: relative;
  max-width: 100%;
  overflow: hidden;
}

.instrument-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 16px;
  font-family: var(--jz-font-mono);
  font-size: 11px;
  letter-spacing: 0.09em;
  padding-bottom: 8px;
}

.instrument-samples {
  display: flex;
  gap: 2px;
  overflow-x: auto;
  scrollbar-width: none;
}

.instrument-samples::-webkit-scrollbar {
  display: none;
}

.instrument-sample {
  font-family: var(--jz-font-mono);
  font-size: 11px;
  letter-spacing: 0.09em;
  color: var(--jz-ink-3);
  background: none;
  border: none;
  padding: 2px 8px;
  cursor: pointer;
  white-space: nowrap;
}

.instrument-sample:hover {
  color: var(--jz-ink);
}

.instrument-sample--active {
  color: var(--jz-amber);
  box-shadow: inset 0 -1px 0 var(--jz-amber);
}

.instrument-readout {
  color: var(--jz-ink-2);
  white-space: nowrap;
}

.instrument-readout b {
  color: var(--jz-amber);
  font-weight: 600;
}

.instrument-pane {
  position: relative;
  height: 420px;
  border: 1px solid var(--jz-line);
  background: var(--jz-panel);
  overflow: hidden;
}

@media (max-width: 640px) {
  .instrument-pane {
    height: 340px;
  }
}

.instrument-error {
  padding: 14px 18px;
  font-family: var(--jz-font-mono);
  font-size: 12px;
  color: var(--jz-danger);
}

.instrument-scale {
  margin-bottom: 10px;
}

.instrument-probe {
  position: absolute;
  visibility: hidden;
  pointer-events: none;
  font-family: var(--jz-font-mono);
  font-size: 13px;
  white-space: pre;
}
</style>
