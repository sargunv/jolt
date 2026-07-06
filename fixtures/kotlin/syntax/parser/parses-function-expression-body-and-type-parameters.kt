fun <T> identity(value: T): T = value

fun <T : Comparable<T>> maxOf(left: T, right: T): T =
    if (left >= right) left else right
