abstract class ModifiedMembers {
    protected abstract val size: Int

    public final fun label(): String = "size=$size"
}
