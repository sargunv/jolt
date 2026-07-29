fun softKeywords(context: Int, field: String, value: Boolean): String {
    val where = context + 1
    val by = field.length
    val all = value && where > by
    return if (all) field else "none"
}

fun softKeywordSafeCall(open: Node?) {
    open?.member()
}

class SoftKeywordCalls(val key: Key) {
    val fromGet = get(key)
    val fromSet = set.tailSet(key)

    val active: Boolean
        get() = get(key)?.active ?: true

    fun calls(set: TreeSet) {
        set("a", "aa")
        val snapshot = cache["a"]!!
        set("b", "bb")
    }

    fun backing(field: Field) {
        val field = field.name
        field.isAccessible = true
        field = field.trim()
        field?.let { consume(it) }
    }
}

class Accessors(var value: Int) {
    var withSetter: Int
        get() = value
        set(newValue) {
            value = newValue
        }

    val withBackingField: Int
        field = value * 2
        get() = field + 1
}

val topLevel: Int
    get() = 42
