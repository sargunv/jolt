import a @A(value;

class FollowingMalformedImport {}

@A(@B)
@interface ValidNestedAnnotation {}

@ @interface RecoveredAnnotationInterface {}

class NestedRecoveryContexts {
  @A @A 0;
  class FollowingMalformedMember {}

  NestedRecoveryContexts int value) @A @A 0;
  class FollowingMalformedConstructor {}
}
