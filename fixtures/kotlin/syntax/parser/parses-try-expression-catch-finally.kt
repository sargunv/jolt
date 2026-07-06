fun tryExpression(path: String): String {
    return try {
        read(path)
    } catch (missing: MissingFile) {
        "missing"
    } catch (error: Throwable) {
        "error"
    } finally {
        close(path)
    }
}
