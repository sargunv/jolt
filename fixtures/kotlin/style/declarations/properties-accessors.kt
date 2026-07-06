package com.example.declarations

class Counter {
    var count: Int = 0
        private set(value) {
            field = value.coerceAtLeast(0)
        }

    val empty: Boolean
        get() = count == 0
}
