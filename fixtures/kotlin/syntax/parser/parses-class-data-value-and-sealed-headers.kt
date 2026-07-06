data class Payload(val bytes: ByteArray)

@JvmInline
value class UserId(val value: String)

sealed class Result
