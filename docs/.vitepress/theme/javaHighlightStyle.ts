import { syntaxHighlighting } from "@codemirror/language";
import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import type { Extension } from "@codemirror/state";

/** Light mode uses basicSetup's defaultHighlightStyle; dark uses One Dark tokens. */
export function javaSyntaxHighlighting(isDark: boolean): Extension | undefined {
  if (!isDark) return undefined;
  return syntaxHighlighting(oneDarkHighlightStyle);
}
