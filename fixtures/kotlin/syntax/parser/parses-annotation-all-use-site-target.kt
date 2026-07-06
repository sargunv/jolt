class AllTarget(
    @all:Label("user")
    val name: String,
)

annotation class Label(val value: String)
