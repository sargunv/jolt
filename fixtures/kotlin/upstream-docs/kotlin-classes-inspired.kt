package com.example.docs

class Customer(val name: String) {
    var visits: Int = 0

    fun greet(): String = "Welcome, $name"
}
