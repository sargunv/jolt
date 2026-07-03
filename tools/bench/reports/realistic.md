# realistic

Spring Framework Java sources.

```text
Benchmark 1: jolt fmt
  Time (abs ≡):         9.942 s               [User: 9.396 s, System: 0.228 s]
 
Benchmark 2: dprint fmt --incremental=false --skip-stable-format
  Time (abs ≡):        845.3 ms               [User: 17299.9 ms, System: 976.4 ms]
 
Benchmark 3: google-java-format --replace
  Time (abs ≡):        28.028 s               [User: 80.176 s, System: 10.477 s]
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (abs ≡):        552.5 ms               [User: 690.5 ms, System: 196.8 ms]
 
Summary
  prettier --write --plugin prettier-plugin-java ran
    1.53 times faster than dprint fmt --incremental=false --skip-stable-format
   17.99 times faster than jolt fmt
   50.73 times faster than google-java-format --replace
```
