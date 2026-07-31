fun pairSum(
  first: Int,
  (a, b): Pair<Int, Int>,
) = first + a + b

val describe = { (
    name,
    age,
) -> "$name is $age" }

val collect = { first: Int,
  (x, y),
  ->
  first + x + y
}
