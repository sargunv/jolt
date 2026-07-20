fun () {
}

fun missingParameters {
}

fun Receiver.member() = value
fun Receiver.() {
}
fun MissingDot member() {
}

fun missingBody() =
val = 1
val delegated by provider
val initialized = value
val (recoveredInitializer) value

val recoveredProperty: Int
    field
    get() value
    set(value) {
        field = value
    } = value

val missingAccessorBody: Int
    get() =

typealias Alias String

class Derived Base(), Other {
    constructor() this()
    constructor():

    val property: Int
        field = 1
        get() = field
        set(value) {
            field = value
        }
}

class EmptyDelegation : {
}

class PartialDelegation : Base by {
}

class OrphanMember {
    +
}

class OrphanComma {
    ,
}

enum class Choice {
    ),
    First(1) {
    },
    Second,,
    Third
}

class MissingClose {
    val value = 1
