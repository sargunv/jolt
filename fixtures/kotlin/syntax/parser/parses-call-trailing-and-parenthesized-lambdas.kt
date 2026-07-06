fun trailing(values: List<Int>) {
    values.map { it + 1 }
        .filter({ value -> value > 2 })
        .fold(0) { acc, value ->
            acc + value
        }
}
