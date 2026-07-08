interface Service {
    fun call(): String
}

class ServiceImpl : Service {
    override fun call(): String = "ok"
}

class DelegatingService(private val service: Service) : Service by service

interface ExpressionValue

interface NumberValue<U> :
    ExpressionValue,
    InterpolatableValue<U>,
    ComparableValue<NumberValue<U>>

class MapNode

class LayerNode : MapNode(), Service by ServiceImpl()
