typealias MissingType =
typealias MissingBeforeModified =
private class AfterMissing
typealias MissingSegment = Alpha.
typealias RangeSeparator = Alpha..Beta
typealias Projections = Box<, String, *String, out, in Number, *>
typealias FunctionParameters = (, named: String, Int) -> Unit
typealias ContextFunction = context(String, , named: Int) (value: String) -> Unit

fun <, T Any, U :> broken(
    first: Int 1,
    ,
    vararg rest: String,
    missingType:,
) T: Any {
}

fun <U> constrained() where U Any {
}

fun <T> constraintGap() where T : Any, , T : Closeable {
}

context(String, named: Int 1, , Other)
fun contextual() {
}
