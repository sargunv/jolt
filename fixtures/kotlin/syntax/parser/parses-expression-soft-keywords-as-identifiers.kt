fun softKeywords(context: Int, field: String, value: Boolean): String {
    val where = context + 1
    val by = field.length
    val all = value && where > by
    return if (all) field else "none"
}
