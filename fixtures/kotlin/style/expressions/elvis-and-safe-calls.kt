package com.example.expressions

class User(val profile: Profile?)
class Profile(val email: String?)

fun email(user: User?): String =
    user?.profile?.email
        ?: "missing@example.com"
