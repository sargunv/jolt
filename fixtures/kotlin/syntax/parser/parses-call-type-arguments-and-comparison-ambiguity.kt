fun typeArguments(a: Int, b: Int, c: Int) {
    val parsed = factory.create<List<String>, Map<String, Int>>(emptyList(), emptyMap())
    val safe = factory.create<Result>()?.value
    val compared = a < b && b > c
    val spaced = value < other > fallback
    val parenthesized = value < (other + fallback)
    val withCall = a < 2 || fallback()
    sink(parsed, safe, compared, spaced, parenthesized, withCall)
}
