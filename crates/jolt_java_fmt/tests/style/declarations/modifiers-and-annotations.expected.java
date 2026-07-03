@Complex( /* args */
  name = "demo", // name
  flags = { 1, 2 }
)
@Generated("tool")
@Deprecated
public final class User {
  @FieldAnno(name = "id", values = { 1, 2 })
  private final String id;
  @FieldOnly
  String unqualified;

  public @Nonnull String name() {
    return id;
  }

  @Action
  @Checked
  public @Nullable void reset() {
  }

  @Override
  String raw() {
    return id;
  }
}

public sealed strictfp class Base permits Open {
}

public non-sealed strictfp class Open extends Base {
}

final /* stable */ public class CommentedModifier {
  volatile /* shared */ public int value;
}
