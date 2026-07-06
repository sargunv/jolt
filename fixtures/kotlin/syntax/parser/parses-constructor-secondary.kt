class SecondaryConstructor private constructor(val name: String) {
    constructor() : this("default")

    constructor(id: Int) : this(id.toString())
}
