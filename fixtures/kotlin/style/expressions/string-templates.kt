package com.example.expressions

fun label(name: String, count: Int): String {
    val plural = if (count == 1) "item" else "items"
    return "$name has ${count + 1} $plural"
}
