val fromWhen = when (a) {
    1 ->
        init(b)
    2 ->
        constructor(c)
    else -> g()
}

val fromInitializer =
    init(b)

val fromConstructorInitializer =
    constructor(b)

fun branches(c: Boolean) {
    if (c)
        init(b)
    else
        constructor(d)
    while (c)
        init(e)
}

val fromLambda = {
    init(b)
}

class MemberCalls(val key: Key) {
    val fromMemberInitializer =
        init(key)

    fun fromMemberBody() =
        init(key)

    init {
        init(key)
    }

    constructor() : this(DefaultKey)
}
