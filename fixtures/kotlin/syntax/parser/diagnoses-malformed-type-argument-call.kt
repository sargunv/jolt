fun malformedTypeArgumentCall(value: String) {
    val broken = factory.create<String,, Int>(value)
    val missing = factory.create<List<String>(value)
    val spacedSafeCall = value? /* gap */ .length
    val spacedTypeArgumentSafeCall = factory.create<Result>? /* gap */ .value
    consume(broken, missing, spacedSafeCall, spacedTypeArgumentSafeCall)
}
