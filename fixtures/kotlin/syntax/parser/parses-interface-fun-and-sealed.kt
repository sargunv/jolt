fun interface Decoder<T> {
    fun decode(text: String): T
}

sealed interface Node
