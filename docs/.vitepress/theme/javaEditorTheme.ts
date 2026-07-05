import { EditorView } from "@codemirror/view";

export function joltEditorTheme(isDark: boolean) {
  return EditorView.theme(
    {
      "&": {
        backgroundColor: "transparent",
        height: "100%",
        maxHeight: "100%",
      },
      ".cm-scroller": {
        overflow: "auto",
        fontFamily: "var(--vp-font-family-mono)",
        fontSize: "13px",
        lineHeight: "1.6",
      },
      ".cm-gutters": {
        backgroundColor: "var(--vp-c-bg-soft)",
        borderRight: "1px solid var(--vp-c-divider)",
        color: "var(--vp-c-text-3)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "var(--vp-c-bg-soft)",
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
        backgroundColor: isDark ? "#6699ff0b" : "rgba(0,0,0,0.03)",
      },
    },
    { dark: isDark },
  );
}
