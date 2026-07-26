interface ExpressionValue

interface EquatableValue

interface MatchableValue

interface AnInterfaceWithAVeryLongNameThatCrowdsOutOtherSupertypes

/** Leads a declaration whose supertype list still fits beside its header. */
sealed interface FormattableValue : ExpressionValue

// Line comment leading a declaration with several supertypes.
sealed interface ComparableValue<T> : ExpressionValue, EquatableValue, MatchableValue

// A supertype list that does not fit still breaks onto its own line.
sealed interface InterpolatableValue<T> :
  ExpressionValue,
  EquatableValue,
  MatchableValue,
  AnInterfaceWithAVeryLongNameThatCrowdsOutOtherSupertypes

fun rank(tag: String): Int = when {
  // Leads a branch whose body still fits beside its arrow.
  tag.startsWith("@return") -> 1
  else -> 2
}
