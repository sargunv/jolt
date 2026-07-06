fun binaryKinds(input: Any?, names: List<String>): String {
    val name = input as? String ?: "guest"
    val present = name in names
    val typed = input is CharSequence && input !is StringBuilder
    return if (present && typed) name else "missing"
}
