context(logger: Logger)
fun logMessage(message: String) {
    logger.write(message)
}

interface Logger {
    fun write(message: String)
}

class ContextMembers {
    context(_: VerificationScope)
    fun verify() = true

    context(scope: Scope)
    val scoped: Boolean
        get() = true
}

enum class ContextEnum {
    Entry;

    context(_: VerificationScope)
    fun verify() = true
}

