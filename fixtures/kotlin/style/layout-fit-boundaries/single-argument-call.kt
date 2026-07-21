package com.example.boundaries

fun validate(name: String) {
  require(name.isNotBlank()) { "blank name" }
}
