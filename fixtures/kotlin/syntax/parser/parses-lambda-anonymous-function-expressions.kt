fun anonymous(values: List<Int>) {
    val block = fun(value: Int): Int {
        return value * 2
    }
    val expression = fun(value: Int): Int = value + 1
    values.map(block).map(expression)
}
