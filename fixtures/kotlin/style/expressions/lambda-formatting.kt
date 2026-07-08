package com.example.expressions

val shout = { value   :   String -> value + "!" }

val sum = { left: Int, right: Int ->
    left+right
}

fun invoke(block: (Int) -> Int): Int = block(1)

val doubled = invoke { value -> value*2 }
