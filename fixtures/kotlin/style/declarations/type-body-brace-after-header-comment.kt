// a comment that ends the header line must not leave a space before the brace
class TopLevelSupertypeComment : Base() // trailing
{
  val x = 1
}

class Outer {
  class NestedSupertypeComment : Base() // trailing
  {
    val z = 3
  }
}
