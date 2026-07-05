<script setup lang="ts">
import { computed } from "vue";
import { java } from "@codemirror/lang-java";
import { linter, type Diagnostic as LintDiagnostic } from "@codemirror/lint";
import CodeMirror from "vue-codemirror6";
import { useData } from "vitepress";
import { joltEditorTheme } from "../javaEditorTheme";

const props = defineProps<{
  modelValue: string;
  readonly?: boolean;
  lintDiagnostics?: LintDiagnostic[];
  lineWidth: number;
  showRuler?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const { isDark } = useData();

const lang = java();
const showRuler = computed(() => props.showRuler ?? true);

const editorStyle = computed(() =>
  showRuler.value
    ? ({ "--jolt-ruler-column": String(props.lineWidth) } as Record<
        string,
        string
      >)
    : undefined,
);

const extensions = computed(() => {
  const extras = [joltEditorTheme(isDark.value)];

  if (props.lintDiagnostics?.length) {
    const diagnostics = props.lintDiagnostics;
    extras.push(linter(() => diagnostics));
  }

  return extras;
});
</script>

<template>
  <CodeMirror
    :model-value="modelValue"
    :lang="lang"
    basic
    :dark="isDark"
    :readonly="readonly"
    :extensions="extensions"
    class="java-editor"
    :class="{ 'java-editor--ruler': showRuler }"
    :style="editorStyle"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>

<style scoped>
.java-editor {
  height: 100%;
}

.java-editor :deep(.cm-editor) {
  height: 100%;
  outline: none;
}

.java-editor :deep(.cm-editor.cm-focused) {
  outline: none;
}

.java-editor--ruler :deep(.cm-content) {
  background-image: linear-gradient(
    90deg,
    transparent calc(var(--jolt-ruler-column) * 1ch - 1px),
    color-mix(in srgb, var(--vp-c-divider) 70%, transparent)
      calc(var(--jolt-ruler-column) * 1ch - 1px),
    color-mix(in srgb, var(--vp-c-divider) 70%, transparent)
      calc(var(--jolt-ruler-column) * 1ch),
    transparent calc(var(--jolt-ruler-column) * 1ch)
  );
  background-attachment: local;
}

.java-editor :deep(.cm-lintRange-error) {
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='6' height='3'%3E%3Cpath d='m0 3 3-3 3 3' fill='%23f66f81'/%3E%3C/svg%3E");
}
</style>
