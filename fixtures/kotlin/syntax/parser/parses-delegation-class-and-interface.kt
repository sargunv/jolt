interface Service {
    fun call(): String
}

class ServiceImpl : Service {
    override fun call(): String = "ok"
}

class DelegatingService(private val service: Service) : Service by service
