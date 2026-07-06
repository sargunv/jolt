const val BuildName = "dev"

var mutableCount: Int = 0

context(config: Config)
val configuredName: String
    get() = config.name

interface Config {
    val name: String
}
