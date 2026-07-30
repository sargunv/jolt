non- // JOLT-TRIVIA:non-sealed-minus-line-top-level
sealed interface TopLevel {}

class NonSealedCrevices {
  non- // JOLT-TRIVIA:non-sealed-minus-line
  sealed interface Member {}

  non // JOLT-TRIVIA:non-sealed-non-line
  - sealed class MemberClass {}

  non /* JOLT-TRIVIA:non-sealed-non-block */ - sealed interface BlockBetween {}

  non- /* JOLT-TRIVIA:non-sealed-minus-block */ sealed interface BlockAfterMinus {}
}

// The same comment is the previous token's trailing trivia when the source keeps
// it on that token's line and the next token's leading trivia when the source
// gives it a line of its own. Both readings have to render identically, or the
// two flip-flop across passes, so these repeat the cases above with the comment
// moved onto its own line.
class NonSealedOwnLineCrevices {
  non-
  // JOLT-TRIVIA:non-sealed-own-line-before-sealed
  sealed interface LineBeforeSealed {}

  non
  // JOLT-TRIVIA:non-sealed-own-line-before-minus
  - sealed class LineBeforeMinus {}

  non-
  /* JOLT-TRIVIA:non-sealed-own-block-before-sealed */
  sealed interface BlockBeforeSealed {}

  non
  /* JOLT-TRIVIA:non-sealed-own-block-before-minus */
  - sealed interface BlockBeforeMinus {}
}
