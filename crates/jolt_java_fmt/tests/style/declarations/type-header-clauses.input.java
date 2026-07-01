class Derived<T> extends /* base */ base.@Marker Parent<String> implements First, // first
pkg.Second permits Child, // child
other.Child {} interface Shape extends Drawable, Scalable permits Circle, Square {} record Point(int x) implements Named, Located {} enum Color implements Named, java.io.Serializable {}
