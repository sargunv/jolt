package com.example.trivia

annotation class Tags(val names: Array<String>)

@Tags([ /* JOLT-TRIVIA:collection-literal-open */ "alpha", "beta" /* JOLT-TRIVIA:collection-literal-close */ ])
class CollectionUse {
    val table = mapOf(
        "one" /* JOLT-TRIVIA:pair-key */ to /* JOLT-TRIVIA:to-call */ 1,
        "two" to 2,
    )
}
