fun conditionWhen(value: Int): String {
    return when (value) {
        0, 1 -> "small"
        in 2..10 -> "medium"
        !in 11..<20 -> "outside"
        else -> "large"
    }
}
