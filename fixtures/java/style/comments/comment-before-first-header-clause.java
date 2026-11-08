interface Manager {
  PersistenceUnitInfo obtainPersistenceUnitInfo(String persistenceUnitName) /* c */
      throws IllegalArgumentException, IllegalStateException;

  void run() /* fits */ throws Exception;
}

class Wide<TheFirstTypeParameter extends Comparable<TheFirstTypeParameter>, Second> /* c */
    extends Base {}
