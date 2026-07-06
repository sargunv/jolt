fun namedDestructuring(user: User) {
    val (name = displayName, age = years) = user
    val (val city = homeCity, var score = points) = user.profile
    consume(displayName, years, homeCity, points)
}
