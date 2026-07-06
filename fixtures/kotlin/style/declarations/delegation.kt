package com.example.declarations

interface Store {
    fun save(value: String)
}

class MemoryStore : Store {
    override fun save(value: String) {}
}

class AuditedStore(private val delegate: Store) : Store by delegate
