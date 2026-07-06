annotation class Mark(val value: String = "")

class TargetedAnnotations {
    @delegate:Mark("lazy")
    val cached: String by lazy { "ready" }

    @set:Mark("setter")
    @setparam:Mark("value")
    var name: String = ""
}
