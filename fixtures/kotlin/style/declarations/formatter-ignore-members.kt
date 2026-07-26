class FormatterIgnoreMembers {
val before=1
  // @formatter:off
  val raw=1+2
  val label="x"
  // @formatter:on
val after=2

  // @formatter:off
  fun   raw( ) { }

  // @formatter:on
  fun formatted() {}

fun nested() {
if (true) {
// @formatter:off
val rawLocal=3+4
call( a,b )
// @formatter:on
val afterLocal=5
}
}
}

interface FormatterIgnoreInterfaceMembers {
// @formatter:off
fun   raw( ): Int
// @formatter:on
fun formatted(): Int
}

enum class FormatterIgnoreEnumEntries {
// @formatter:off
  RAW  ,
// @formatter:on
  FORMATTED,
}

object FormatterIgnoreObject {
// @formatter:off
val raw=1
// @formatter:on
val formatted=2
}
