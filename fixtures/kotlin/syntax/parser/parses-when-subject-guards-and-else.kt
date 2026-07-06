fun guardedWhen(token: Token): String {
    return when (token) {
        is Token.Word if token.text.isNotEmpty() -> "word"
        is Token.Number if token.value > 0 -> "number"
        null -> "missing"
        else -> "other"
    }
}
