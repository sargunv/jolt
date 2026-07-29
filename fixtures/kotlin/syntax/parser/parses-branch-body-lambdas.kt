fun branchLambdas(v: Int): (Int) -> String {
    val fromWhen = when (v) {
        1 -> { _: Int -> "one" }
        2 -> { it -> it.toString() }
        else -> { it -> "other" }
    }
    val fromIf = if (v == 1) { _: Int -> "one" } else { it -> "other" }
    return fromWhen
}

fun branchBlocks(v: Int) {
    if (v == 1) {
        println("one")
    }
    when (v) {
        1 -> { println("one") }
        else -> { println("other") }
    }
    while (v == 1) {
        break
    }
    for (i in 1..v) {
        println(i)
    }
}
