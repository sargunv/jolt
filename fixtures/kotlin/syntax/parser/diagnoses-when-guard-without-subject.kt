fun invalidGuardWithoutSubject(value: Int): String {
    return when {
        value > 0 if value < 10 -> "small"
        else -> "other"
    }
}
