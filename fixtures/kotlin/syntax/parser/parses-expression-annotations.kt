fun annotatedExpressions(flag: Boolean) {
    val environment =
        @Suppress("OPT_IN_USAGE_ERROR") // createForProduction uses an internal opt-in
        createEnvironment()

    val choice =
        @Suppress("DEPRECATION")
        when (flag) {
            true -> "yes"
            false -> "no"
        }

    val length =
        @Suppress("DEPRECATION")
        when (flag) {
            true -> "yes"
            false -> "no"
        }.length
}
