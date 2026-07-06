fun safeChain(node: Node?): String {
    return node
        ?.parent
        ?.children
        ?.get(0)
        ?.name!!
}
