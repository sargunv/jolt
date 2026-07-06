package com.example.trivia

fun destructure(entries: List<Pair<String, Int>>) {
    for ((name /* JOLT-TRIVIA:first-component */ , count) in /* JOLT-TRIVIA:in-keyword */ entries) {
        val (first, second /* JOLT-TRIVIA:second-component */) = name to count
        println("$first:$second")
    }
}
