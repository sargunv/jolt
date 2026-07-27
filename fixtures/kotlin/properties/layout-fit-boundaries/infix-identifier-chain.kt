fun infixChains(baseValue: Int, shiftAmountFromConfiguration: Int, lowBitsMaskValue: Int, highMaskValue: Int) {
  val mask = baseValue shl shiftAmountFromConfiguration or lowBitsMaskValue and highMaskValue
  for (index in initialIndexValue until exclusiveEndIndexBoundary step configuredStepSizeValue) {
    consume(index)
  }
}
