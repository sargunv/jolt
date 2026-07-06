class A {
  ;
  // keep class marker
  ;
  ; /* keep trailing class marker */
  ;
}
interface Api {
  ; // keep interface marker
  void call(); // call marker
}
@interface Marker {
  /* keep annotation marker */ ;
  String value();
}
enum Mode {
  A;
  // keep enum marker
  ;
  void run() {}
}
enum EmptyEnum {
  ; // keep empty enum marker
}
