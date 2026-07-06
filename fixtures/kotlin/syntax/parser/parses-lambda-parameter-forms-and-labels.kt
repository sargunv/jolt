fun lambdas(values: List<String>) {
    val implicit = values.map { it.length }
    val typed = values.map { value: String -> value.trim() }
    val labeled = values.forEach label@{
        if (it.isEmpty()) return@label
        consume(it)
    }
    val block = {
        val maxLine: (String) -> Int = { text ->
            if (text.indexOf(' ') == -1) {
                0
            } else {
                text.length
            }
        }
        maxLine
    }
    val functionTyped = values.map { transform: (String) -> Int ->
        transform("value")
    }
}
