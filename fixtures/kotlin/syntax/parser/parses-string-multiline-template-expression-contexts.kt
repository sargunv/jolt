fun multilineTemplates(total: Int): String {
    val raw = """
        total=${total + 1}
        status=${if (total > 0) "ready" else "empty"}
    """.trimIndent()
    val multiDollar = $$"""
        literal $name
        interpolated $$total
    """
    return raw + multiDollar
}
