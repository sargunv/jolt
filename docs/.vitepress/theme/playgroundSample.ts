/** Default playground input: LRU cache in a non-Jolt style (4-space, Allman braces). */
export const PLAYGROUND_SAMPLE_JAVA = `import java.util.Map;
import java.util.HashMap;

public class LruCache<K,V>
{
    private final int capacity;
    private final Map<K,Node<K,V>> nodes=new HashMap<>();
    private final Node<K,V> head=new Node<>(null,null);
    private final Node<K,V> tail=new Node<>(null,null);

    public LruCache(int capacity){
        this.capacity=capacity;
        head.next=tail; tail.prev=head;
    }

    public V get(K key)
    {
        Node<K,V> node=nodes.get(key);
        if(node==null) return null;
        moveToFront(node);
        return node.value;
    }

    public void put(K key,V value){
        Node<K,V> node=nodes.get(key);
        if(node!=null){
            node.value=value; moveToFront(node); return;
        }
        node=new Node<>(key,value);
        nodes.put(key,node);
        insertAfter(head,node);
        if(nodes.size()>capacity){
            Node<K,V> lru=removeBefore(tail);
            nodes.remove(lru.key);
        }
    }

    private void moveToFront(Node<K,V> node){remove(node);insertAfter(head,node);}

    private void remove(Node<K,V> node){
        node.prev.next=node.next;
        node.next.prev=node.prev;
    }

    private void insertAfter(Node<K,V> anchor,Node<K,V> node)
    {
        node.next=anchor.next; node.prev=anchor;
        anchor.next.prev=node; anchor.next=node;
    }

    private Node<K,V> removeBefore(Node<K,V> anchor){
        Node<K,V> node=anchor.prev;
        remove(node);
        return node;
    }

    private static final class Node<K,V>{
        final K key; V value;
        Node<K,V> prev,next;
        Node(K key,V value){this.key=key;this.value=value;}
    }

    public static void main(String[] args)
    {
        LruCache<String,Integer> cache=new LruCache<>(2);
        cache.put("a",1); cache.put("b",2);
        System.out.println(cache.get("a"));
        cache.put("c",3);
        System.out.println(cache.get("b"));
        System.out.println(cache.get("c"));
    }
}
`;
