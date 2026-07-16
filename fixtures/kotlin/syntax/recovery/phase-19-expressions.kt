fun missingRight() = 1 +
val invalidTarget = 1 = 2
fun missingSelector() = target.
fun missingReference() = target::
fun argumentGap() = call(, value)
fun emptyArgumentGap() = call(,)
fun indexGap() = value[, index]
fun lambdaGap() = { , value -> value }
fun emptyLambdaGap() = { , -> 1 }
fun emptyParenthesized() = ()
fun collectionGap() = [, value]
fun emptyCollectionGap() = [,]
fun stringTemplate() = "value: $value"
fun anonymous() = fun(value: Int) = value
val missingAnonymousBody = fun()
val missingAnonymousParameters = fun {}
val validAnonymousExpressionBody = fun() = 1
val validAnonymousBlockBody = fun() {}
fun objectLiteral() = object : Base by delegate {
}
val missingObjectBody = object
val missingDelegatedObjectBody = object : Base
val validObjectBody = object {}
val validDelegatedObjectBody = object : Base {}
val missingThisLabel = this@
val missingSuperLabel = super@
val validThisLabel = this@owner
val validSuperLabel = super<Base>@owner
fun missingCollectionClose() = [value
