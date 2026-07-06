fun destructuring(pair: Pair<String, Int>, entries: List<Pair<String, Int>>) {
    val (name, count) = pair
    var (mutableName, mutableCount) = pair
    for ((key, value) in entries) {
        consume(key, value, name, count, mutableName, mutableCount)
    }
}
