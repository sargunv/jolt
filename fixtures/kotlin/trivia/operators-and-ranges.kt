package com.example.trivia

fun operators(values: List<Int>): Boolean {
    val range = 1 /* JOLT-TRIVIA:range-left */ .. /* JOLT-TRIVIA:range-op */ 10
    val indexed = values[0 /* JOLT-TRIVIA:index-value */]
    return indexed in /* JOLT-TRIVIA:in-operator */ range && indexed !in /* JOLT-TRIVIA:not-in-operator */ setOf<Int>()
}
