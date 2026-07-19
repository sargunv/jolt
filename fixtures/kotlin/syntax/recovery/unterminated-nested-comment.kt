class UnterminatedNestedComment {
fun complete() { val value=1 }
}
/* outer unterminated
 * before nested /* nested closes */
 * outer remains open
