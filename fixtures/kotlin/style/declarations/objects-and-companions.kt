package com.example.declarations

object Registry {
    private val values = mutableMapOf<String, String>()

    fun put(key: String, value: String) {
        values[key] = value
    }
}

class WithCompanion {
    companion object {
        const val KIND = "fixture"
    }
}
