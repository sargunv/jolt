val String.lastIndexOrZero: Int
    get() = length - 1

val com.example.Scope.qualifiedTitle: String
    get() = toString()

val MutableMap<String, Expression<*>>.entryCount: Int
    get() = size
