import type { Diagnostic as LintDiagnostic } from "@codemirror/lint";

export type FormatDiagnostic = {
  code: string;
  severity: string;
  message: string;
  line: number;
  column: number;
  from: number;
  to: number;
};

const DIAGNOSTIC_RE =
  /code=(?<code>\S+)\s+severity=(?<severity>\S+)\s+stage=\S+\s+message=(?<message>.*?)\s+line=(?<line>\d+)\s+column=(?<column>\d+)\s+range=(?<from>\d+)\.\.(?<to>\d+)/g;

export function parseFormatError(error: string): FormatDiagnostic[] {
  const diagnostics: FormatDiagnostic[] = [];

  for (const match of error.matchAll(DIAGNOSTIC_RE)) {
    const groups = match.groups;
    if (!groups) {
      continue;
    }

    diagnostics.push({
      code: groups.code,
      severity: groups.severity,
      message: groups.message,
      line: Number(groups.line),
      column: Number(groups.column),
      from: Number(groups.from),
      to: Number(groups.to),
    });
  }

  return diagnostics;
}

export function toLintDiagnostics(
  diagnostics: FormatDiagnostic[],
): LintDiagnostic[] {
  const byRange = new Map<string, LintDiagnostic>();

  for (const diagnostic of diagnostics) {
    const from = diagnostic.from;
    const to = Math.max(diagnostic.to, from + 1);
    const key = `${from}:${to}`;
    const severity =
      diagnostic.severity === "error"
        ? "error"
        : diagnostic.severity === "warning"
          ? "warning"
          : "info";

    const existing = byRange.get(key);
    if (existing) {
      existing.message = `${existing.message}; ${diagnostic.message}`;
      continue;
    }

    byRange.set(key, {
      from,
      to,
      severity,
      message: diagnostic.message,
    });
  }

  return [...byRange.values()];
}

export function formatErrorSummary(error: string): string {
  const diagnostics = parseFormatError(error);
  if (diagnostics.length > 0) {
    const first = diagnostics[0];
    return `Line ${first.line}, column ${first.column}: ${first.message}`;
  }

  return error;
}
