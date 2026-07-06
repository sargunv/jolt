package com.example.declarations

class Location {
    val city: String
        field = "Paris"

    var country: String
        field = "FR"
        get() = field.uppercase()
        set(value) {
            field = value.trim()
        }
}
