package com.example.expressions

fun callChains(source: Sequence<String>): List<String> =
    source
        .map { it.trim() }
        .filter { it.isNotEmpty() }
        .sorted()
        .toList()
