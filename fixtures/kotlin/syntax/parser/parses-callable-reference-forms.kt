fun callableReferences() {
    val top = ::topLevel
    val member = String::trim
    val bound = "value"::length
    val klass = String::class
    consume(top, member, bound, klass)
}
