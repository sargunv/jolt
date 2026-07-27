fun trailingComments(values: List<Int>, chain: Chain) {
  val numbers = listOf(1, 2, 3) // declaration
  values.map { it + 1 } // lambda
  chain.first().second() // chain
}
