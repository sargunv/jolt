<script setup lang="ts">
import { computed } from "vue";
import { java } from "@codemirror/lang-java";
import { linter, type Diagnostic as LintDiagnostic } from "@codemirror/lint";
import type { LanguageSupport } from "@codemirror/language";
import CodeMirror from "vue-codemirror6";
import { useData } from "vitepress";
import { joltEditorTheme } from "../javaEditorTheme";
import { joltSyntaxHighlighting } from "../javaHighlightStyle";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    language?: LanguageSupport;
    readonly?: boolean;
    lintDiagnostics?: LintDiagnostic[];
    lineWidth: number;
    showRuler?: boolean;
  }>(),
  { language: () => java(), showRuler: true },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const { isDark } = useData();

const editorStyle = computed(() =>
  props.showRuler
    ? ({ "--jolt-ruler-column": String(props.lineWidth) } as Record<
        string,
        string
      >)
    : undefined,
);

const extensions = computed(() => {
  const extras = [
    joltSyntaxHighlighting(isDark.value),
    joltEditorTheme(isDark.value),
  ];

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
    :lang="language"
    basic
    :dark="isDark"
    :readonly="readonly"
    :extensions="extensions"
    class="source-editor"
    :class="{ 'source-editor--ruler': showRuler }"
    :style="editorStyle"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>

<style scoped>
.source-editor {
  position: absolute;
  inset: 0;
}

.source-editor :deep(.vue-codemirror),
.source-editor :deep(.cm-editor) {
  height: 100%;
  max-height: 100%;
  outline: none;
}

.source-editor :deep(.cm-editor.cm-focused) {
  outline: none;
}

.source-editor--ruler :deep(.cm-content) {
  background-image: linear-gradient(
    90deg,
    transparent calc(var(--jolt-ruler-column) * 1ch - 1px),
    color-mix(in srgb, var(--jz-amber) 45%, transparent)
      calc(var(--jolt-ruler-column) * 1ch - 1px),
    color-mix(in srgb, var(--jz-amber) 45%, transparent)
      calc(var(--jolt-ruler-column) * 1ch),
    transparent calc(var(--jolt-ruler-column) * 1ch)
  );
  background-attachment: local;
}

.source-editor :deep(.cm-lintRange-error) {
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='6' height='3'%3E%3Cpath d='m0 3 3-3 3 3' fill='%23b3261e'/%3E%3C/svg%3E");
}
</style>
