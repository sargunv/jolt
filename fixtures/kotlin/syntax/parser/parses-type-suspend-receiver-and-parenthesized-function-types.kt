typealias Suspended = suspend (String) -> Int

typealias ReceiverBlock = String.(Int) -> Unit

typealias NullableCallback = ((String?) -> Unit)?

val receiverBlock: ReceiverBlock = { count -> repeat(count) { trim() } }
