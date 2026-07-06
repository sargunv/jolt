@Route(path = "/items", methods = ["GET", "POST"])
class AnnotatedRoute

annotation class Route(
    val path: String,
    val methods: Array<String>,
)
