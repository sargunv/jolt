package com.example.boundaries

fun boundary(source: Sequence<String>): List<String> =
    source.map { it.trim().lowercase().replace("-", "_").removePrefix("tmp_") }.filter { it.isNotEmpty() }.toList()
