typealias T=@
typealias AnnotatedSuspend = @A suspend () -> Unit
typealias AnnotatedContext = @A context(String) () -> Unit

fun f(x: Any) = when(x) { is @ String -> 1 }

class Following
