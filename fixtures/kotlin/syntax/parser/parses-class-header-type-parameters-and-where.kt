class Registry<K, V>(
    private val values: Map<K, V>,
) where K : CharSequence, K : Comparable<K>, V : Any
