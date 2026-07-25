class TrailingCommentSpacing {
  val a = 1 // trailing
  val b = 2

  val c = 3 // trailing

  val d = 4
  fun e() {} // trailing
  fun f() {}
  val g = 5 /* block does not end the line */
  val h = 6
}

interface I {
  fun a() // trailing
  fun b()
}

enum class E {
  A, // trailing
  B,
}
