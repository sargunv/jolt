fun newlineSelectors(receiver: Receiver) {
    val selected = receiver
        .child
        .call()

    receiver
    standalone()

    consume(selected)
}
