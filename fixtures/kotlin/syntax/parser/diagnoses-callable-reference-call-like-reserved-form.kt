fun invalidCallableReferenceCall() {
    val direct = String::trim(" value ")
    val generic = Box::create<String>("value")
    consume(direct, generic)
}
