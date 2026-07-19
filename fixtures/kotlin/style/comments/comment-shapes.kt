/**/
class CommentShapes {
/**/
val field=0

fun emptyBlocks() {
/**/
/***/
/** */
/* */
/*
*/
/*
 *
 */
/**
 */
/**
 *
 */
}

fun adjacentBlocks() {
/*alpha*//*beta*/
val first/**/=1
val second/***/=2
val third/* ***/=3
/* outer /* inner */ outer */
}

fun delimiterComments() {
call(/**/)
call(/* */)
}

fun lineComments() {
//
val value=1
}

fun singleEmptyBlock() { /**/ }

fun singleStarDoc() { /*****/ }

fun singleNestedBlock() { /* outer /* inner */ outer */ }

fun singleLine() { //
}

fun call(vararg values: Any?) {}
}//