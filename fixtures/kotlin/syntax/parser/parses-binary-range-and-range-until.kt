fun ranges(limit: Int): List<Int> {
    val closed = 0..limit
    val open = 0..<limit
    val stepped = 10 downTo 0 step 2
    return closed.filter { it in open } + stepped.toList()
}
