fun redundantSemicolons() {
  val first = 1; val second = 2
  consume(first); consume(second)
}

fun lambdaDeclarationSeparators() {
  consume {
    val first = 1;
    val second = 2;
    use(first, second)
  }
}
