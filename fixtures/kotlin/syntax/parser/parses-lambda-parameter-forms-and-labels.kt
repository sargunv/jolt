fun lambdas(values: List<String>) {
    val implicit = values.map { it.length }
    val typed = values.map { value: String -> value.trim() }
    val labeled = values.forEach label@{
        if (it.isEmpty()) return@label
        consume(it)
    }
}
