fun loops(values: Iterable<String>) {
    while (hasNext()) advance()
    do {
        tick()
    } while (pending())

    for (value in values) {
        consume(value)
    }

    for ((index, value) in values.withIndex()) {
        consume(index, value)
    }
}
