typealias Callback = context(Logger) (String?) -> Unit

val callback: Callback = { value -> println(value) }

interface Logger
