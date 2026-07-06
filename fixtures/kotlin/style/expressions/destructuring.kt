package com.example.expressions

fun totals(entries: List<Pair<String, Int>>): Map<String, Int> {
    val result = mutableMapOf<String, Int>()
    for ((name, count) in entries) {
        result[name] = (result[name] ?: 0) + count
    }
    return result
}
