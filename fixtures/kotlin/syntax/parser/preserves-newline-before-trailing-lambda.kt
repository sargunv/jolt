fun value(): Int = 1

fun nextLineBlock() {
  value()
  { println() }
}
