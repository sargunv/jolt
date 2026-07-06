fun templates(user: User, count: Int): String {
    val simple = "Hello, $user"
    val braced = "Next count is ${count + 1}"
    val nested = "Name: ${user.name ?: "anonymous"}"
    return "$simple; $braced; $nested"
}
