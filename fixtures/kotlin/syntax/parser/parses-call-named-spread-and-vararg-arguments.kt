fun callArguments(items: Array<String>) {
    target(
        first = "one",
        second = compute(),
        *items,
        last = true,
    )
}
