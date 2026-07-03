# adversarial

google-java-format formatter test inputs.

```text
Benchmark 1: jolt fmt
  Time (abs ≡):         64.6 ms               [User: 56.8 ms, System: 6.6 ms]
 
Benchmark 2: dprint fmt --incremental=false --skip-stable-format
  Time (abs ≡):         31.7 ms               [User: 94.6 ms, System: 53.6 ms]
 
Benchmark 3: google-java-format --replace
  Time (abs ≡):        192.0 ms               [User: 372.8 ms, System: 204.9 ms]
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (abs ≡):        432.5 ms               [User: 547.2 ms, System: 140.2 ms]
 
Summary
  dprint fmt --incremental=false --skip-stable-format ran
    2.04 times faster than jolt fmt
    6.05 times faster than google-java-format --replace
   13.63 times faster than prettier --write --plugin prettier-plugin-java
```
