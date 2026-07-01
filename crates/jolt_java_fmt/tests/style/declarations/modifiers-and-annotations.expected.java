@Complex( /* args */
  name = "demo", // name
  flags = {1, 2}
)
@Generated("tool")
@Deprecated
public final class User {
  @FieldAnno(name = "id", values = {1, 2})
  private final String id;

  public @Nonnull String name() {
    return id;
  }

  @Action
  @Checked
  public @Nullable void reset() {
  }
}
