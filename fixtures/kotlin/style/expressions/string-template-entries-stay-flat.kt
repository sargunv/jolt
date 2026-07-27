fun stringTemplateEntriesStayFlat(user: Long, pendingCredits: Long, pendingDebits: Long) {
  val line = "user has ${user + pendingCredits + pendingDebits + user + user + pendingCredits} money"
  val raw = """user has ${user + pendingCredits + pendingDebits + user + user + pendingCredits} money"""
  val call = "user has ${accounting.service().nestedCall(user, pendingCredits, pendingDebits, user)} money"
}
