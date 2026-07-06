fun unary(counter: Counter): Int {
    var index = 0
    val before = ++index + counter.value--
    val after = -before + +index
    return if (!counter.done) after else before
}
