fun processWhile() {
  while (shouldKeepProcessing()) processTheNextAvailableItemUsingAllConfiguredHandlers()
}

fun processFor(values: List<String>) {
  for (value in values) processTheCurrentValueWithAllConfiguredTransformations(value)
}

fun processDoWhile() {
  do processTheNextAvailableItemUsingAllConfiguredHandlers() while (shouldKeepProcessing())
}

fun returnChoice(flag: Boolean): String {
  return if (flag) computeSomethingLongForTheTrueBranch() else computeSomethingEntirelyDifferentForTheFalseBranch()
}

fun returnParenthesizedChoice(flag: Boolean): String {
  return (if (flag) computeSomethingLongForTheTrueBranch() else computeSomethingEntirelyDifferentForTheFalseBranch())
}
