fun validControlFlow(value: Any, ready: Boolean, items: List<Any>) {
    ;
    if (ready); else {}
    when (value) {
        1, -> one
        else -> zero
    }
    try {} catch (cause: Throwable) {} finally {}
    for (item in items) {}
    while (ready);
    do {} while (ready)
    return@owner value
}

fun missingIfParts() {
    if value
    val afterIf = 1
}

fun missingWhenBraces(value: Int) {
    when (value)
        1 -> one
        2 -> two
    val afterWhen = 1
}

fun malformedWhenEntries(value: Int) {
    when (value) {
        , 1 -> one
        2 two
        3 -> three
        4 if -> four
        else ->
    }
}

fun malformedTry() {
    try
    catch {}
    finally {}
    catch (late: Throwable) {}
}

fun malformedLoops(items: List<Any>, ready: Boolean) {
    for (in items)
    while
    do {} (ready)
}

fun malformedJumps() {
    break value
    continue value
    return@
    throw
    val afterThrow = 1
}

fun missingNestedBlockClose() {
    if (true) {
        while (true) {
            value
