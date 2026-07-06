fun choose(a: Int, b: Int, c: Int): Int {
    return if (a > b)
        if (a > c) a else c
    else
        if (b > c) b else c
}
