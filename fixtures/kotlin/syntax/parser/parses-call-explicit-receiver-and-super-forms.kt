open class Base {
    open fun call(value: String) {}
}

class Derived : Base() {
    override fun call(value: String) {
        this.call(value = value.trim())
        super.call(value)
    }
}
