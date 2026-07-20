import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

/**
 * Palette syntax highlighting for editors: ink for identifiers, steel for
 * keywords, amber for strings, teal for numbers, faded ink for comments.
 * Same hues as the Shiki css-variables theme in custom.css.
 */
export function joltSyntaxHighlighting(isDark: boolean): Extension {
  const ink = isDark ? "#ece8dc" : "#1c1a13";
  const ink2 = isDark ? "#98927f" : "#57544a";
  const ink3 = isDark ? "#6e695b" : "#8a8577";
  const steel = isDark ? "#8fb4ec" : "#2d5da8";
  const teal = isDark ? "#6fd3c6" : "#0e7470";
  const string = isDark ? "#ffb224" : "#8f5e00";

  return syntaxHighlighting(
    HighlightStyle.define([
      { tag: [t.keyword, t.modifier, t.controlKeyword], color: steel },
      { tag: [t.string, t.special(t.string), t.docString], color: string },
      { tag: [t.number, t.bool, t.null, t.atom], color: teal },
      { tag: [t.comment, t.blockComment, t.lineComment], color: ink3, fontStyle: "italic" },
      { tag: [t.typeName, t.className, t.tagName], color: ink, fontWeight: "500" },
      { tag: [t.function(t.variableName), t.function(t.propertyName)], color: ink },
      { tag: [t.propertyName, t.attributeName], color: ink },
      { tag: [t.variableName], color: ink },
      { tag: [t.annotation, t.meta], color: ink2 },
      { tag: [t.operator], color: ink2 },
      { tag: [t.punctuation, t.bracket, t.paren], color: ink2 },
    ]),
  );
}
