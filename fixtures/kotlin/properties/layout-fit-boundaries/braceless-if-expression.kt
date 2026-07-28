fun shortChoice(flag: Boolean) = if (flag) first() else second()

fun chooseValue(flag: Boolean) = if (flag) computeSomethingLongForTheTrueBranch() else computeSomethingEntirelyDifferentForTheFalseBranch()

fun commentedChoice(flag: Boolean) = if (flag) computeSomethingLongForTheTrueBranch() // then branch
else computeSomethingEntirelyDifferentForTheFalseBranch()

fun blockChoice(flag: Boolean) = if (flag) {
  first()
} else {
  second()
}

fun nestedChoice(first: Boolean, second: Boolean) = if (first) firstValue() else if (second) secondValue() else thirdValue()

class Choice {
  fun chooseValue(flag: Boolean) = if (flag) computeSomethingLongForTheTrueBranch() else computeSomethingEntirelyDifferentForTheFalseBranch()
}

fun failChoice(flag: Boolean): Nothing = throw if (flag) buildSomethingLongForTheTrueFailure() else buildSomethingEntirelyDifferentForTheFalseFailure()
