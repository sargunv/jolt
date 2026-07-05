import java.util.List;
;
class Helper {String join(){return "x";}}
String greeting="Hello";
void main(){IO.println(greeting+List.of(new Helper().join()));}
interface Named{String name();}
record Pair(String left,String right){}
enum Mode{FAST,SLOW}
@interface Marker{String value() default "ok";}
final class Tail{}
