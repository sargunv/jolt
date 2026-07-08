annotation class Composable
annotation class Fancy(val name: String)

fun render(
    menu: @Composable () -> Unit,
    map: @Composable (PaddingValues) -> Unit,
    trailingContent: @Composable (() -> Unit)? = null,
    decorated: @Fancy("label") String,
) = Unit

class PaddingValues
