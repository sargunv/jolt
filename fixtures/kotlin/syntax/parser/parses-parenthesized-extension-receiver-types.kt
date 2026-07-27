fun ((Int) -> Int).twice(value: Int) = value * 2

fun (Int).identity() = this

val (Int).property: Int
  get() = this
