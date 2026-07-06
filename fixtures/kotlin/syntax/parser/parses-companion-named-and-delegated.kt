interface Provider {
    fun provide(): String
}

class WithCompanion {
    companion object Defaults : Provider {
        override fun provide(): String = "default"
    }
}
