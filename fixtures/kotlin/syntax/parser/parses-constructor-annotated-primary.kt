class Injected @Inject constructor(
    @Named("id") val id: String,
)

annotation class Inject
annotation class Named(val value: String)
