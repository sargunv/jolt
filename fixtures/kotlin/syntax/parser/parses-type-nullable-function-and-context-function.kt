typealias Callback = context(Logger) (String?) -> Unit
typealias NamedCallback = (value: String?, count: Int) -> Unit

val callback: Callback = { value -> println(value) }

interface Logger
