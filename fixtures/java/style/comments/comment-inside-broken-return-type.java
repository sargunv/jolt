class CommentInsideBrokenReturnType {
  Pair< /* a comment inside a return type that breaks */
          Pair<Pair<Pair<T, T>, Pair<T, T>>, Pair<Pair<T, T>, Pair<T, T>>>,
          Pair<Pair<Pair<T, T>, Pair<T, T>>, Pair<Pair<T, T>, Pair<T, T>>>>
      method(int x) throws Exception {
    return null;
  }
}
