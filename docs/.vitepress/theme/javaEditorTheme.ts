import { EditorView } from "@codemirror/view";

export function joltEditorTheme(isDark: boolean) {
  return EditorView.theme(
    {
      "&": {
        backgroundColor: "transparent",
        height: "100%",
      },
      ".cm-scroller": {
        fontFamily: "var(--vp-font-family-mono)",
        fontSize: "13px",
        lineHeight: "1.6",
      },
      ".cm-gutters": {
        backgroundColor: "transparent",
        borderRight: "1px solid var(--vp-c-divider)",
        color: "var(--vp-c-text-3)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "transparent",
      },
      ".cm-content": {
        caretColor: "var(--vp-c-brand-1)",
        padding: "12px 0",
      },
      ".cm-cursor": {
        borderLeftColor: "var(--vp-c-brand-1)",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
        backgroundColor: "var(--vp-c-brand-soft) !important",
      },
      ".cm-activeLine": {
        backgroundColor: isDark ? "rgba(255,255,255,0.04)" : "rgba(0,0,0,0.03)",
      },
    },
    { dark: isDark },
  );
}
