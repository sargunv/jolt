// A comment trailing the last segment of a qualified name sits after the whole
// name, so the name stays compact rather than splitting across lines.
package a.b.c // trailing the package name

import kotlin.collections.List // trailing the import name

fun names(values: List<String>): String {
  return values.joinToString()
}
