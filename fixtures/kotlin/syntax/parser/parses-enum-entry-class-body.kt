enum class Token {
    Word {
        override fun text(): String = "word"
    },
    Number {
        override fun text(): String = "number"
    };

    abstract fun text(): String
}
