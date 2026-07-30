// A destructuring entry begins with an optional `val`/`var` and a name, so a
// literal there is something no entry can consume: `parse_name` reports the
// missing name without advancing. The entry loop has to take the token itself or
// it never advances -- unbounded in a release build, where the debug assertion
// that guards the loop is compiled out.
fun literalEntries() {
  val (1, 2) = pair
}

fun mixedEntries() {
  val (first, 2, third) = triple
}

fun trailingLiteralEntry() {
  val (first, 2) = pair
}

fun bracketedLiteralEntries() {
  val [1, 2] = pair
}
