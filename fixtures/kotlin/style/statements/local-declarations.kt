package com.example.statements

fun localFactory(prefix: String): () -> String {
    data class Local(val value: String)
    val local = Local(prefix.trim())
    return { local.value }
}
