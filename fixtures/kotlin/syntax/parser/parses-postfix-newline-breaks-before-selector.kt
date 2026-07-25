fun newlineSelectors(receiver: Receiver) {
    val selected = receiver
        .child
        .call()

    receiver
    standalone()

    consume(selected)
}

fun newlineEndsTightSuffixes(receiver: Receiver) {
    receiver
    ["key"]

    receiver.child
    { it.active }

    receiver
    ::child
}
