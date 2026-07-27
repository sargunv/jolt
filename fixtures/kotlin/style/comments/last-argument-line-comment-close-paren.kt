fun lastArgumentComment() {
  consume(
    first,
    second // last argument
  )
}

fun trailingCommaComment() {
  consume(
    first,
    second, // trailing comma owns the comment
  )
}
