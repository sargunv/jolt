package com.example.statements

fun firstMatch(rows: List<List<String>>, needle: String): String? {
    outer@ for (row in rows) {
        for (cell in row) {
            if (cell == needle) break@outer
        }
    }
    return null
}
