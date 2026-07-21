package sample

import sample.Z
// @formatter:off
import   sample.KeepUgly
import   sample.AlsoKeepUgly
// @formatter:on
import sample.A

// @formatter:off
class   KeepUgly   {   fun   messy( )  { println(1) } }
// @formatter:on
class FormatMe {
  fun ok() {
    println(2)
  }
}

class Body {
  // @formatter:off
  val    kept=1
  fun    ugly( ) { println( 2 ) }
  // @formatter:on
  fun good() {
    println(3)
  }
}

fun block() {
  // @formatter:off
  println(   "kept" )
  if (true) { println(1) }
  // @formatter:on
  println(4)
}

fun terminalBlock() {
  // @formatter:off
  val retries  = 3
  val timeout  = 5000
  // @formatter:on
}

class TerminalBody {
  // @formatter:off
  val retries  = 3
  val timeout  = 5000
  // @formatter:on
}
