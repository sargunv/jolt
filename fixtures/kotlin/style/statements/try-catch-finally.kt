package com.example.statements

fun parseOrZero(text: String): Int {
    return try {
        text.trim().toInt()
    } catch (error: NumberFormatException) {
        0
    } finally {
        println("parsed")
    }
}
