class ConstructorProperty(val constructor: Any) {
    init {
        constructor.toString()
    }

    fun reassign() {
        constructor(this)
        val copy = constructor
    }
}

class SecondaryConstructor(val x: Int) {
    constructor() : this(0)

    @Inject
    private constructor(y: Long) : this(y.toInt())
}
