package com.example.docs

fun describe(value: Any): String =
    when (value) {
        is String -> "text with ${value.length} chars"
        is Int -> "number $value"
        else -> "unknown"
    }
