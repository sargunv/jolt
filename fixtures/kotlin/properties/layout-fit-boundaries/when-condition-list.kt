fun classifyCondition(value: Any): Int = when (value) {
  is VeryLongConditionTypeNumberOne, is VeryLongConditionTypeNumberTwo, is VeryLongConditionTypeNumberThree -> 1
  else -> 2
}

fun classifyCommentedCondition(value: Any): Int = when (value) {
  is FirstType, // first alternative
  is SecondType,
  is ThirdType -> 1
  else -> 2
}
