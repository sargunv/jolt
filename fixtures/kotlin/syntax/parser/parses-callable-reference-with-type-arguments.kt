fun callableReferenceTypeArguments() {
    val generic = Box::create<String>
    val nested = Registry.Entry::from<Int>
    consume(generic, nested)
}
