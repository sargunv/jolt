class Example {
  boolean disjoint(Rectangle bounds) {
    return !(
      bounds.getLeft() > getRight()
        || bounds.getRight() < getLeft()
        || bounds.getTop() > getBottom()
        || bounds.getBottom() < getTop()
    );
  }
}
