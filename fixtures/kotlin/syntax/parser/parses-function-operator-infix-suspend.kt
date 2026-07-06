class Counter(val value: Int) {
    operator fun plus(other: Counter): Counter = Counter(value + other.value)

    infix fun above(other: Counter): Boolean = value > other.value
}

suspend fun fetchValue(): String = "value"
