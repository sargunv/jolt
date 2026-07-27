fun classifyCondition(value: Any): Int = when (value) {
  is VeryLongConditionTypeNumberOne, is VeryLongConditionTypeNumberTwo, is VeryLongConditionTypeNumberThree -> 1
  else -> 2
}
