<script setup lang="ts">
import { java } from "@codemirror/lang-java";
import { LanguageSupport } from "@codemirror/language";
import type { Diagnostic as LintDiagnostic } from "@codemirror/lint";
import { computed, onMounted, reactive, ref, watch } from "vue";
import SourceEditor from "./SourceEditor.vue";
import { useJoltFormatter } from "../composables/useJoltFormatter";
import { kotlinLanguage } from "../kotlinLanguage";
import {
  PLAYGROUND_DEFAULT_CONFIG,
  type PlaygroundFormatConfig,
} from "../playgroundConfig";
import {
  PLAYGROUND_SAMPLE_JAVA,
  PLAYGROUND_SAMPLE_KOTLIN,
} from "../playgroundSample";
import {
  formatErrorSummary,
  parseFormatError,
  toLintDiagnostics,
} from "../parseFormatError";

type PlaygroundLanguage = "java" | "kotlin";

type PlaygroundLanguageOption = {
  id: PlaygroundLanguage;
  label: string;
  filePath: string;
  editorLanguage: LanguageSupport;
};

type PlaygroundSession = {
  input: string;
  output: string;
  formatError: string | null;
  formatOk: boolean;
  formatGeneration: number;
};

const PLAYGROUND_LANGUAGES: readonly PlaygroundLanguageOption[] = [
  {
    id: "java",
    label: "Java",
    filePath: "Example.java",
    editorLanguage: java(),
  },
  {
    id: "kotlin",
    label: "Kotlin",
    filePath: "Example.kt",
    editorLanguage: new LanguageSupport(kotlinLanguage),
  },
];

const languageById = Object.fromEntries(
  PLAYGROUND_LANGUAGES.map((language) => [language.id, language]),
) as Record<PlaygroundLanguage, PlaygroundLanguageOption>;

const sessions = reactive<Record<PlaygroundLanguage, PlaygroundSession>>({
  java: {
    input: PLAYGROUND_SAMPLE_JAVA,
    output: "",
    formatError: null,
    formatOk: false,
    formatGeneration: 0,
  },
  kotlin: {
    input: PLAYGROUND_SAMPLE_KOTLIN,
    output: "",
    formatError: null,
    formatOk: false,
    formatGeneration: 0,
  },
});

const activeLanguage = ref<PlaygroundLanguage>("java");
const config = ref<PlaygroundFormatConfig>({ ...PLAYGROUND_DEFAULT_CONFIG });

const { loading, loadError, ensureReady, formatSource } = useJoltFormatter();

const language = computed(() => languageById[activeLanguage.value]);
const session = computed(() => sessions[activeLanguage.value]);
const input = computed({
  get: () => session.value.input,
  set: (value: string) => {
    session.value.input = value;
  },
});
const output = computed(() => session.value.output);
const formatError = computed(() => session.value.formatError);
const formatOk = computed(() => session.value.formatOk);

const inputLintDiagnostics = computed<LintDiagnostic[]>(() => {
  if (!formatError.value) {
    return [];
  }

  return toLintDiagnostics(parseFormatError(formatError.value));
});

const formatStatus = computed(() => {
  if (loading.value) {
    return "loading";
  }
  if (formatError.value) {
    return "error";
  }
  if (formatOk.value) {
    return "ok";
  }
  return "idle";
});

const formatStatusTitle = computed(() => {
  if (formatError.value) {
    return formatErrorSummary(formatError.value);
  }
  if (formatOk.value) {
    return "Formatted successfully";
  }
  return undefined;
});

async function runFormat(
  languageId: PlaygroundLanguage,
  source: string,
  formatConfig: PlaygroundFormatConfig,
) {
  const targetSession = sessions[languageId];
  const generation = ++targetSession.formatGeneration;
  targetSession.formatError = null;
  targetSession.formatOk = false;

  try {
    const result = await formatSource(
      languageById[languageId].filePath,
      source,
      formatConfig,
    );
    if (generation === targetSession.formatGeneration) {
      targetSession.output = result;
      targetSession.formatOk = true;
    }
  } catch (error) {
    if (generation === targetSession.formatGeneration) {
      targetSession.output = "";
      targetSession.formatError =
        error instanceof Error ? error.message : String(error);
      targetSession.formatOk = false;
    }
  }
}

watch(
  [activeLanguage, input, config],
  ([languageId, value, formatConfig]) => {
    if (loading.value || loadError.value) {
      return;
    }
    void runFormat(languageId, value, formatConfig);
  },
  { deep: true },
);

onMounted(async () => {
  try {
    await ensureReady();
    await runFormat(activeLanguage.value, input.value, config.value);
  } catch {
    // loadError is surfaced in the template
  }
});
</script>

<template>
  <section class="jolt-playground">
    <p v-if="loadError" class="jolt-playground-load-error">
      Failed to load formatter: {{ loadError }}
    </p>

    <div class="jolt-playground-panels">
      <div class="jolt-playground-panel">
        <div class="jolt-playground-panel-label">
          <span class="jolt-playground-panel-title">
            <span>Input</span>
            <span
              v-if="!loading"
              class="jolt-playground-status"
              :class="`jolt-playground-status--${formatStatus}`"
              :title="formatStatusTitle"
              aria-hidden="true"
            />
          </span>

          <div
            class="jolt-playground-languages"
            role="group"
            aria-label="Playground language"
          >
            <button
              v-for="option in PLAYGROUND_LANGUAGES"
              :key="option.id"
              type="button"
              class="jolt-playground-language"
              :class="{
                'jolt-playground-language--active':
                  option.id === activeLanguage,
              }"
              :aria-pressed="option.id === activeLanguage"
              @click="activeLanguage = option.id"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <div class="jolt-playground-editor">
          <div v-if="loading" class="jolt-playground-loading">Loading…</div>
          <SourceEditor
            v-else
            :key="`input-${activeLanguage}`"
            v-model="input"
            :accessible-name="`${language.label} source input`"
            :language="language.editorLanguage"
            :line-width="config.lineWidth"
            :show-ruler="false"
            :lint-diagnostics="inputLintDiagnostics"
          />
        </div>
      </div>

      <div class="jolt-playground-panel">
        <div class="jolt-playground-panel-label">
          <span class="jolt-playground-panel-title">
            <span>Formatted</span>
          </span>

          <div v-if="!loading && !loadError" class="jolt-playground-controls">
            <label class="jolt-playground-control">
              <span class="jolt-playground-control-label">Line width</span>
              <input
                v-model.number="config.lineWidth"
                class="jolt-playground-input"
                type="number"
                min="40"
                max="120"
                step="1"
              />
            </label>

            <label class="jolt-playground-control">
              <span class="jolt-playground-control-label">Indent</span>
              <select
                v-model.number="config.indentWidth"
                class="jolt-playground-input"
                :disabled="config.useTabs"
              >
                <option :value="2">2</option>
                <option :value="4">4</option>
              </select>
            </label>

            <label
              class="jolt-playground-control jolt-playground-control--checkbox"
            >
              <input v-model="config.useTabs" type="checkbox" />
              <span class="jolt-playground-control-label">Tabs</span>
            </label>
          </div>
        </div>
        <div class="jolt-playground-editor">
          <div v-if="loading" class="jolt-playground-loading">Loading…</div>
          <SourceEditor
            v-else
            :key="`output-${activeLanguage}`"
            :model-value="output"
            :accessible-name="`Formatted ${language.label} output`"
            :language="language.editorLanguage"
            :line-width="config.lineWidth"
            show-ruler
            readonly
          />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.jolt-playground {
  --jolt-playground-gap: 16px;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  width: 100%;
}

.jolt-playground-languages {
  display: inline-flex;
  border: 1px solid var(--jz-line);
}

.jolt-playground-language {
  height: 22px;
  border: 0;
  border-left: 1px solid var(--jz-line);
  padding: 0 10px;
  font: inherit;
  letter-spacing: inherit;
  text-transform: inherit;
  color: var(--jz-ink-2);
  background: var(--jz-paper);
  cursor: pointer;
}

.jolt-playground-language:first-child {
  border-left: 0;
}

.jolt-playground-language:hover {
  color: var(--jz-ink);
}

.jolt-playground-language:focus-visible {
  position: relative;
  outline: 2px solid var(--jz-amber);
  outline-offset: 1px;
  z-index: 1;
}

.jolt-playground-language--active,
.jolt-playground-language--active:hover {
  color: var(--jz-paper);
  background: var(--jz-ink);
}

.jolt-playground-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.jolt-playground-control {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-family: var(--jz-font-mono);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--jz-ink-2);
}

.jolt-playground-control--checkbox {
  gap: 6px;
}

.jolt-playground-control-label {
  white-space: nowrap;
}

.jolt-playground-input {
  width: 3.5rem;
  height: 22px;
  box-sizing: border-box;
  border: 1px solid var(--jz-line);
  border-radius: 0;
  padding: 0 6px;
  font-size: 11px;
  line-height: 20px;
  font-family: var(--jz-font-mono);
  color: var(--jz-ink);
  background: var(--jz-paper);
}

.jolt-playground-input[type="number"] {
  padding-right: 2px;
}

.jolt-playground-control--checkbox input {
  width: 13px;
  height: 13px;
  margin: 0;
}

.jolt-playground-input:focus {
  outline: 2px solid color-mix(in srgb, var(--jz-amber) 40%, transparent);
  border-color: var(--jz-amber);
}

.jolt-playground-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

@media (max-width: 768px) {
  .jolt-playground-controls {
    justify-content: flex-start;
  }
}

.jolt-playground-load-error {
  margin: 0 0 12px;
  font-size: 14px;
  color: var(--jz-danger);
}

.jolt-playground-panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 380px;
  gap: var(--jolt-playground-gap);
  min-height: 0;
  overflow: hidden;
}

@media (max-width: 768px) {
  .jolt-playground-panels {
    grid-template-columns: 1fr;
    grid-template-rows: 320px 320px;
  }
}

.jolt-playground-panel {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--jz-line);
  background: var(--jz-panel);
  overflow: hidden;
}

.jolt-playground-panel-label {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--jz-line);
  font-family: var(--jz-font-mono);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: var(--jz-ink-2);
  flex-shrink: 0;
}

.jolt-playground-panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.jolt-playground-status {
  width: 8px;
  height: 8px;
  flex-shrink: 0;
}

.jolt-playground-status--idle,
.jolt-playground-status--loading {
  background: var(--jz-ink-3);
}

.jolt-playground-status--ok {
  background: var(--jz-amber);
}

.jolt-playground-status--error {
  background: var(--jz-danger);
}

.jolt-playground-editor {
  position: relative;
  flex: 1;
  min-height: 220px;
  overflow: hidden;
}

.jolt-playground-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  font-family: var(--jz-font-mono);
  font-size: 13px;
  color: var(--jz-ink-3);
}
</style>
