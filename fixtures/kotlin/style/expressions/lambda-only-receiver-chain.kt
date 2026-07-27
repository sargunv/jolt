fun lambdaOnlyReceiver(items: List<Int>) {
  items.forEach { println(it) }
  items.map { it + 1 }.filter { it > 1 }
}
