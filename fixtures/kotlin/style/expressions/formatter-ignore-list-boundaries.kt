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
