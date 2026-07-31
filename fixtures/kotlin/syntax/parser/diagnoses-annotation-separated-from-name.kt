class SeparatedAnnotations {
    val product = 5 * factor

    @ Volatile
    var counter = 0

    @/* a comment is trivia too */JvmField
    var backing: String = "initial"

    @
    Volatile
    var detached = 1

    @ get:Rule
    val rule = ExecutorRule(4)

    @ [Deprecated("x") Suppress("y")]
    var multi = 2
}

val separatedExpression = @ Suppress("x") compute()
