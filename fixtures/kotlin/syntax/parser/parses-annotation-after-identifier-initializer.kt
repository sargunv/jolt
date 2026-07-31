class AnnotationAfterIdentifierInitializer {
    val product = 5 * factor

    @Volatile
    var counter = 0

    var selfRef = this

    @PublishedApi
    internal var other = this

    val state: String get() = backing

    // used by reflective callers
    @Volatile
    @JvmField
    var backing: String = "initial"
}

fun topLevel(): Int = other eq this

// NOT EQUAL

@LowPriority
fun annotated() {}

fun labels() {
    loop@ for (i in 1..10) {
        if (i > 1) break@loop
        continue@loop
    }
    val labeled = run inner@{ 1 }
    consume(this@labels, labeled)
}

private val iterations = 50_000 * stressTestMultiplier

@get:Rule
val pool = ExecutorRule(4)
