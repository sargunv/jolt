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
        fontFamily: "var(--jz-font-mono)",
        fontSize: "13px",
        lineHeight: "1.75",
      },
      ".cm-gutters": {
        backgroundColor: "var(--jz-panel)",
        borderRight: "1px solid var(--jz-line)",
        color: "var(--jz-ink-3)",
        fontFamily: "var(--jz-font-mono)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "var(--jz-panel-inset)",
        color: "var(--jz-ink-2)",
      },
      ".cm-content": {
        caretColor: "var(--jz-amber)",
        padding: "14px 0",
      },
      ".cm-cursor": {
        borderLeftColor: "var(--jz-amber)",
        borderLeftWidth: "2px",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
        backgroundColor: isDark
          ? "rgba(255, 214, 10, 0.22) !important"
          : "rgba(255, 207, 63, 0.5) !important",
      },
      ".cm-activeLine": {
        backgroundColor: isDark ? "rgba(255, 178, 36, 0.05)" : "rgba(0,0,0,0.025)",
      },
    },
    { dark: isDark },
  );
}
