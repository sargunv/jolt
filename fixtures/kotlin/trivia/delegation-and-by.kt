package com.example.trivia

interface Logger {
    fun log(message: String)
}

class ConsoleLogger : Logger {
    override fun log(message: String) {}
}

class Service(
    private val logger: Logger,
) : Logger /* JOLT-TRIVIA:delegation-type */ by /* JOLT-TRIVIA:delegation-by */ logger

val cached: String /* JOLT-TRIVIA:property-delegate-type */ by lazy /* JOLT-TRIVIA:property-by */ {
    "value"
}
