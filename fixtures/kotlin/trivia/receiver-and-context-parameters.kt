package com.example.trivia

class HtmlScope {
    fun text(value: String) {}
}

context(/* JOLT-TRIVIA:context-open */ scope: HtmlScope /* JOLT-TRIVIA:context-param */)
fun String /* JOLT-TRIVIA:receiver-type */ .render(/* JOLT-TRIVIA:receiver-dot */) {
    scope.text(this)
}

val HtmlScope /* JOLT-TRIVIA:property-receiver */ .title: String
    get() = "ready"
