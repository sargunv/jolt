@ /*a*/ Anno
class SigilComment

@file /*b*/ :JvmName("X")

@file: /*c*/ JvmName("Y")

val objectExpression = object /*d*/ : B() {}

val suspendType: suspend /*e*/ () -> Unit = {}

val anonymous = fun /*f*/ (x: Int) = x

val receiverAnonymous = fun Int. /*g*/ (): Int = this

fun labeled() { l@ /*h*/ { g() } }

fun spread(a: A) { g(* /*i*/ a) }

class Variance<in /*j*/ A>

val projection: List<out /*k*/ Any> = f()
