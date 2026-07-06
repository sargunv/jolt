fun parenthesized(flag: Boolean, value: String?): String {
    val chosen = (if (flag) value else "fallback") ?: "empty"
    val recovered = (try {
        chosen.trim()
    } catch (error: Throwable) {
        "error"
    })
    return recovered
}
