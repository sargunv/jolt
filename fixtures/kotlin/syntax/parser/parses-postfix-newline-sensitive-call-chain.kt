fun chained(source: Source): Result {
    return source
        .select("name")
        .filter { it.active }
        [0]
        ?.build()
        ?: Result.Empty
}
