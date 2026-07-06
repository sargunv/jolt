fun expressionJumps(value: String?): String {
    val checked = value ?: throw MissingValue()
    return checked.takeIf { it.isNotEmpty() } ?: return "empty"
}
