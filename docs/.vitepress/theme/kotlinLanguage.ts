import { StreamLanguage, type StreamParser } from "@codemirror/language";

/**
 * Minimal Kotlin tokenizer for docs demos. Handles the constructs that show
 * up in sample code: keywords, strings with templates, comments, numbers,
 * annotations, and type names. Not a full grammar.
 */

const KEYWORDS: Record<string, true> = Object.fromEntries(
  `package import as class interface object enum data sealed open abstract final override
   fun val var constructor init companion this super if else when for while do return
   break continue in is null true false by out vararg suspend private public internal
   protected typealias where get set inline reified const operator infix`
    .split(/\s+/)
    .map((word) => [word, true]),
);

type KotlinState = {
  blockCommentDepth: number;
  inString: boolean;
};

const kotlinParser: StreamParser<KotlinState> = {
  name: "kotlin",

  startState: () => ({ blockCommentDepth: 0, inString: false }),

  token(stream, state) {
    if (state.blockCommentDepth > 0) {
      if (stream.match("/*")) {
        state.blockCommentDepth++;
      } else if (stream.match("*/")) {
        state.blockCommentDepth--;
      } else {
        stream.next();
      }
      return "comment";
    }

    if (state.inString) {
      if (stream.eat("\\")) {
        stream.next();
        return "string";
      }
      if (stream.eat('"')) {
        state.inString = false;
        return "string";
      }
      if (stream.match("${")) {
        return "string";
      }
      stream.next();
      return "string";
    }

    if (stream.eatSpace()) return null;

    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match("/*")) {
      state.blockCommentDepth = 1;
      return "comment";
    }
    if (stream.eat('"')) {
      state.inString = true;
      return "string";
    }
    if (stream.eat("@")) {
      stream.match(/^[A-Za-z_][\w.]*/);
      return "meta";
    }
    if (stream.match(/^0[xXbB][\da-fA-F_]+[lL]?/) || stream.match(/^\d[\d_]*(\.\d+)?[fFlL]?/)) {
      return "number";
    }
    if (stream.match(/^[A-Z][\w]*/)) {
      return "typeName";
    }
    if (stream.match(/^[a-z_][\w]*/)) {
      const word = stream.current();
      return KEYWORDS[word] ? "keyword" : "variableName";
    }
    if (stream.match(/^[+\-*/%=<>!&|?:.]+/)) {
      return "operator";
    }

    stream.next();
    return null;
  },

  languageData: {
    commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
    closeBrackets: { brackets: ["(", "[", "{", '"'] },
  },
};

export const kotlinLanguage = StreamLanguage.define(kotlinParser);
