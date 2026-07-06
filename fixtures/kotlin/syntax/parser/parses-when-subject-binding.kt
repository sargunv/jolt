fun subjectBinding(input: Any): Int {
    return when (val text = input as? String) {
        null -> 0
        else -> text.length
    }
}
