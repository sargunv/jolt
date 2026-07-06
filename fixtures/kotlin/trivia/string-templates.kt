package com.example.trivia

fun templates(user: String, count: Int): String {
    val simple = "Hello, ${ /* JOLT-TRIVIA:template-name */ user }"
    val nested = "Total: ${ /* JOLT-TRIVIA:template-open */ count + 1 /* JOLT-TRIVIA:template-expression */ }"
    return "$simple; $nested"
}
