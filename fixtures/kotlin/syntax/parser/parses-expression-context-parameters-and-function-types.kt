context(logger: Logger)
fun withContextParameter(value: String): Unit = logger.info(value)

typealias Handler = context(Session) (String) -> Unit

fun functionType(handler: Handler) {
    handler("ready")
}
