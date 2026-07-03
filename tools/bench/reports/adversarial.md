# adversarial

google-java-format formatter test inputs.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):      63.9 ms ±   0.7 ms    [User: 57.0 ms, System: 5.6 ms]
  Range (min … max):    63.4 ms …  64.7 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      30.5 ms ±   1.6 ms    [User: 93.0 ms, System: 50.5 ms]
  Range (min … max):    29.4 ms …  32.3 ms    3 runs
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     212.1 ms ±  10.4 ms    [User: 400.7 ms, System: 187.4 ms]
  Range (min … max):   200.5 ms … 220.7 ms    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     449.0 ms ±  10.5 ms    [User: 541.0 ms, System: 172.2 ms]
  Range (min … max):   439.9 ms … 460.5 ms    3 runs
 
Summary
  dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format ran
    2.10 ± 0.11 times faster than jolt fmt
    6.96 ± 0.50 times faster than google-java-format --replace
   14.74 ± 0.85 times faster than prettier --write --plugin prettier-plugin-java
```
