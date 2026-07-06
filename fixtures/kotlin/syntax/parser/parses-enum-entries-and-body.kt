enum class Direction(val dx: Int, val dy: Int) {
    North(0, -1),
    South(0, 1);

    fun vertical(): Boolean = dx == 0
}
