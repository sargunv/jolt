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

/** Kotlin translation of the Java playground sample, with the same intentionally non-Jolt style. */
export const PLAYGROUND_SAMPLE_KOTLIN = `class LruCache<K : Any,V>(private val capacity:Int)
{
    private val nodes=HashMap<K,Node<K,V>>()
    private val head=Node<K,V>(null,null)
    private val tail=Node<K,V>(null,null)

    init{
        head.next=tail; tail.prev=head
    }

    fun get(key:K):V?
    {
        val node=nodes[key] ?: return null
        moveToFront(node)
        return node.value
    }

    fun put(key:K,value:V){
        var node=nodes[key]
        if(node!=null){
            node.value=value; moveToFront(node); return
        }
        node=Node(key,value)
        nodes[key]=node
        insertAfter(head,node)
        if(nodes.size>capacity){
            val lru=removeBefore(tail)
            lru.key?.let(nodes::remove)
        }
    }

    private fun moveToFront(node:Node<K,V>){remove(node);insertAfter(head,node)}

    private fun remove(node:Node<K,V>){
        node.prev!!.next=node.next
        node.next!!.prev=node.prev
    }

    private fun insertAfter(anchor:Node<K,V>,node:Node<K,V>)
    {
        node.next=anchor.next; node.prev=anchor
        anchor.next!!.prev=node; anchor.next=node
    }

    private fun removeBefore(anchor:Node<K,V>):Node<K,V>{
        val node=anchor.prev!!
        remove(node)
        return node
    }

    private class Node<K,V>(val key:K?,var value:V?){
        var prev:Node<K,V>?=null
        var next:Node<K,V>?=null
    }
}

fun main()
{
    val cache=LruCache<String,Int>(2)
    cache.put("a",1); cache.put("b",2)
    println(cache.get("a"))
    cache.put("c",3)
    println(cache.get("b"))
    println(cache.get("c"))
}
`;
