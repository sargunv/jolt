fun String.surrounded(prefix: String, suffix: String): String =
    prefix + this + suffix

fun PsiElement?.containsNewline(): Boolean = textContains('\n')

fun com.example.Scope.renderQualified(): String = toString()

fun MutableMap<String, Expression<*>>.record(key: String, value: Expression<*>) {
    put(key, value)
}
