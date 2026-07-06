class Accessors {
    var name: String = ""
        get() = field.ifBlank { "unknown" }
        private set(value) {
            field = value.trim()
        }
}
