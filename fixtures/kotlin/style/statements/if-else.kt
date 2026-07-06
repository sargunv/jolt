package com.example.statements

fun choose(value: Int): String {
    if (value < 0) {
        return "negative"
    } else if (value == 0) {
        return "zero"
    }
    return "positive"
}
