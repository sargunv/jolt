val markerBeforeComma = listOf(
  // @formatter:off
  1   +   2
  // @formatter:on
  , trailingValue,
)

val inlineMarkerBeforeComma = listOf(
  // @formatter:off
  1   +   2 /* @formatter:on */, trailingValue,
)

val inlineMarkerBeforeLaterBlockComment = listOf(
  // @formatter:off
  1   +   2 /* @formatter:on */ /* later */, trailingValue,
)

val inlineMarkerBeforeLaterLineComment = listOf(
  // @formatter:off
  1   +   2 /* @formatter:on */ // later
  , trailingValue,
)

val ignoredFinalArgument = listOf(
  // @formatter:off
  first   +   second
  // @formatter:on
)

val adjacentIgnoredRuns = listOf(
  // @formatter:off
  first   +   value
  // @formatter:on
  ,
  // @formatter:off
  second   +   value
  // @formatter:on
  , trailingValue,
)

val ignoredLambdaParameters = { // @formatter:off
  first   :   Int
  // @formatter:on
  , second: Int -> first + second
}

val ownLineIgnoredLambdaParameters = {
  // @formatter:off
  first   :   Int
  // @formatter:on
  , second: Int -> first + second
}

val ignoredFinalLambdaParameter = {
  // @formatter:off
  value   :   Int
  // @formatter:on
  -> value
}

fun <T> ignoredTypeConstraints(value: T): T
  where // @formatter:off
    T   :   FirstConstraint
    // @formatter:on
    , T : SecondConstraint = value

fun <T> ownLineIgnoredTypeConstraints(value: T): T
  where
    // @formatter:off
    T   :   FirstConstraint
    // @formatter:on
    , T : SecondConstraint = value

fun ignoredWhenConditions(value: Any): Int = when (value) {
  // @formatter:off
  is   FirstType
  // @formatter:on
  , is SecondType -> 1
  else -> 0
}

fun ignoredFinalWhenCondition(value: Any): Int = when (value) {
  // @formatter:off
  is   FirstType
  // @formatter:on
  -> 1
  else -> 0
}

fun ignoredFinalGuardedCondition(value: Any, allowed: Boolean): Int = when (value) {
  // @formatter:off
  is   FirstType
  // @formatter:on
  if allowed -> 1
  else -> 0
}

class IgnoredDelegation :
  // @formatter:off
  FirstType   ,   SecondType
  // @formatter:on
{
}
