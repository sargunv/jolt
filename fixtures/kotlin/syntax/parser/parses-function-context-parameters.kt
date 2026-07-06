context(logger: Logger)
fun logMessage(message: String) {
    logger.write(message)
}

interface Logger {
    fun write(message: String)
}
