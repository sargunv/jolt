fun callableReferences() {
    val top = ::topLevel
    val member = String::trim
    val bound = "value"::length
    val klass = String::class
    consume(top, member, bound, klass)
}

fun nullableTypeReceivers() {
    val simple = String?::plus
    val generic = List<String>?::plus
    val qualified = Map.Entry?::key
    val nestedArguments = Map<String, List<Int>>?::keys
    consume(simple, generic, qualified, nestedArguments)
}
