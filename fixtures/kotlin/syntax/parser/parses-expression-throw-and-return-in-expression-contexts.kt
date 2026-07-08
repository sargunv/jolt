fun expressionJumps(value: String?): String {
    val checked = value ?: throw MissingValue()
    return checked.takeIf { it.isNotEmpty() } ?: return "empty"
}

fun guardReturn(isOpen: Boolean) {
    if (!isOpen) return
    val afterGuard = Unit
}
