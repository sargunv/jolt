class Derived<T>
  extends base.@Marker Parent<String>
  implements First, pkg.Second
  permits Child, other.Child {
}

interface Shape extends Drawable, Scalable permits Circle, Square {
}

record Point(int x) implements Named, Located {
}

enum Color implements Named, java.io.Serializable {
}
