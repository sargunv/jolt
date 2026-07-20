public class LruCache<.K,V>
class Fields {
  int .value;
  private final Map<K,..Node<K,V>> nodes=new HashMap<>();
  private final Map<K,+Node<K,V>> alternateNodes=new HashMap<>();
}
