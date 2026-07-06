package com.example.expressions

fun render(items: List<String>, emit: (String) -> Unit) {
    items.forEach { item ->
        emit(item.uppercase())
    }
}

val joined = listOf("a", "b", "c").joinToString(separator = ",") { it.uppercase() }
