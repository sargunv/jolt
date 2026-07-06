package com.example.statements

fun log(value: Any?) {
    when (value) {
        null -> println("null")
        is String -> println(value.length)
        else -> println(value)
    }
}
