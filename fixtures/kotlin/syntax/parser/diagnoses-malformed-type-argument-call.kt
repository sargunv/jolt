fun malformedTypeArgumentCall(value: String) {
    val broken = factory.create<String,, Int>(value)
    val missing = factory.create<List<String>(value)
    consume(broken, missing)
}
