import joltWasmUrl from "../assets/jolt-plugin.wasm?url";
import {
  createContext,
  type ContextFormatter,
  type FormatterContext,
} from "@dprint/formatter";
import { ref, shallowRef } from "vue";
import type { PlaygroundFormatConfig } from "../playgroundConfig";

const context = shallowRef<FormatterContext | null>(null);
const plugin = shallowRef<ContextFormatter | null>(null);
const loading = ref(false);
const loadError = ref<string | null>(null);

let loadPromise: Promise<ContextFormatter> | null = null;

async function ensureReady(): Promise<ContextFormatter> {
  if (plugin.value) {
    return plugin.value;
  }

  if (loadPromise) {
    return loadPromise;
  }

  loading.value = true;
  loadError.value = null;

  loadPromise = (async () => {
    const ctx = createContext();
    const formatter = await ctx.addPluginStreaming(fetch(joltWasmUrl));
    context.value = ctx;
    plugin.value = formatter;
    return formatter;
  })()
    .catch((error: unknown) => {
      loadPromise = null;
      loadError.value =
        error instanceof Error ? error.message : String(error);
      throw error;
    })
    .finally(() => {
      loading.value = false;
    });

  return loadPromise;
}

async function formatSource(
  filePath: string,
  source: string,
  config: PlaygroundFormatConfig,
): Promise<string> {
  const formatter = await ensureReady();
  return formatter.formatText({
    filePath,
    fileText: source,
    overrideConfig: {
      lineWidth: config.lineWidth,
      indentWidth: config.indentWidth,
      useTabs: config.useTabs,
    },
  });
}

export function useJoltFormatter() {
  return { loading, loadError, ensureReady, formatSource };
}
