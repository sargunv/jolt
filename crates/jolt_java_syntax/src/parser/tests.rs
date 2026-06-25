// Java SE 26 grammar and syntax specification:
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-2.html
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-19.html
//
// Java parser focused-test bar. Focused tests should cover:
//
// - every Java syntax grammar declaration, using small representative programs
//   rather than every possible production combination;
// - every known parser ambiguity, including contextual keywords, type-vs-
//   expression boundaries, lambda parameters, casts, patterns, switch labels,
//   and greater-than token splitting in type contexts;
// - error recovery shapes when a diagnostic or recovery boundary is part of
//   parser behavior the formatter depends on.
// - regression tests grounded in actual bugs we have written
//
// Focused tests should not try to enumerate the combinatorial product of the
// grammar. Each test should make one source-shape claim obvious.
