@Complex(/* args */ name="demo", // name
flags={1,2}) @Generated("tool") @Deprecated final public class User { @FieldAnno(name="id", values={1,2}) private final String id; @FieldOnly String unqualified; public @Nonnull String name(){ return id; } @Action @Checked public @Nullable void reset(){} @Override String raw(){ return id; } }
strictfp sealed public class Base permits Open {}
strictfp non-sealed public class Open extends Base {}
final /* stable */ public class CommentedModifier { volatile /* shared */ public int value; }
