fun guardedConditions(value: Any): String =
    when (value) {
        0, 1 if value.hashCode() >= 0 -> "small"
        is String, is CharSequence if value.toString().isNotEmpty() -> "text"
        else -> "other"
    }
