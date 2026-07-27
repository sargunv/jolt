fun binaryOperatorsBreakSafely(
  aLongVariableName: Int,
  anotherLongVariableName: Int,
  yetAnother: Int,
  lastOne: Int,
) {
  val subtraction = aLongVariableName - anotherLongVariableName - yetAnother - lastOne - lastOne
  val multiplication = aLongVariableName * anotherLongVariableName * yetAnother * lastOne * lastOne
  val comparison = aLongVariableName == anotherLongVariableName && yetAnother == lastOne
}
