import type { Diagnostic as LintDiagnostic } from "@codemirror/lint";

/** One `line:column: code: message` line of a plugin format error. */
export type FormatDiagnostic = {
  line: number;
  column: number;
  code: string;
  message: string;
};

const DIAGNOSTIC_RE = /^(\d+):(\d+): (\S+): (.*)$/gm;

export function parseFormatError(error: string): FormatDiagnostic[] {
  return [...error.matchAll(DIAGNOSTIC_RE)].map((match) => ({
    line: Number(match[1]),
    column: Number(match[2]),
    code: match[3],
    message: match[4],
  }));
}

/** Marks each diagnostic's position in `source`, merging shared positions. */
export function toLintDiagnostics(
  diagnostics: FormatDiagnostic[],
  source: string,
): LintDiagnostic[] {
  const lineStarts = [0];
  for (let i = 0; i < source.length; i++) {
    if (source[i] === "\n") lineStarts.push(i + 1);
  }

  const byOffset = new Map<number, LintDiagnostic>();
  for (const diagnostic of diagnostics) {
    const lineStart = lineStarts[diagnostic.line - 1] ?? source.length;
    const from = Math.min(lineStart + diagnostic.column - 1, source.length);
    const existing = byOffset.get(from);
    if (existing) {
      existing.message = `${existing.message}; ${diagnostic.message}`;
    } else {
      byOffset.set(from, {
        from,
        to: Math.min(from + 1, source.length),
        severity: "error",
        message: diagnostic.message,
      });
    }
  }
  return [...byOffset.values()];
}

export function formatErrorSummary(error: string): string {
  const [first] = parseFormatError(error);
  return first
    ? `Line ${first.line}, column ${first.column}: ${first.message}`
    : error;
}
