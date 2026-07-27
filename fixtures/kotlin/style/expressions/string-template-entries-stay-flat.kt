fun stringTemplateEntriesStayFlat(user: Long, pendingCredits: Long, pendingDebits: Long) {
  val line = "user has ${user + pendingCredits + pendingDebits + user + user + pendingCredits} money"
  val raw = """user has ${user + pendingCredits + pendingDebits + user + user + pendingCredits} money"""
  val call = "user has ${accounting.service().nestedCall(user, pendingCredits, pendingDebits, user)} money"
  val trailingCommaCall = "value=${foo(1,)}"
  val rawTrailingCommaCall = """value=${foo(1,)}"""
  val trailingLambdaChain = "value=${items.map { it + 1 }.filter { it > 1 }}"
  val rawTrailingLambdaChain = """value=${items.map { it + 1 }.filter { it > 1 }}"""
}
