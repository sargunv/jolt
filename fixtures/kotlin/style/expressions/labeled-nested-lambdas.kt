package com.example.expressions

fun visit(groups: List<List<String>>, emit: (String) -> Unit) {
    groups.forEach outer@{ group ->
        group.forEach inner@{ item ->
            if (item.isBlank()) return@inner
            if (item == "stop") return@outer
            emit(item.trim())
        }
    }
}
