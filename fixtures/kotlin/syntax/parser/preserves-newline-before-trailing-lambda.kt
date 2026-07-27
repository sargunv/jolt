fun returnsInt(): Int = 1

fun acceptsLambda(block: () -> Unit) {}

fun acceptsTwo(first: () -> Unit, second: () -> Unit) {}

fun nextLineTrailingLambdas() {
  returnsInt()
  { println("ambiguous") }
  acceptsLambda()
  { println("valid") }
  acceptsLambda() // boundary comment
  { println("commented") }
  acceptsLambda()
  /* leading comment */ { println("leading comment") }
  acceptsTwo()
  first@ { println("first") }
  second@ { println("second") }
  acceptsLambda() { println("same line") }
}
