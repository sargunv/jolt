# realistic

Spring Framework Java sources.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):     10.319 s ±  0.034 s    [User: 9.443 s, System: 0.347 s]
  Range (min … max):   10.293 s … 10.358 s    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     847.1 ms ±   6.7 ms    [User: 17065.3 ms, System: 1114.4 ms]
  Range (min … max):   843.0 ms … 854.8 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     11.191 s ±  0.288 s    [User: 64.210 s, System: 7.374 s]
  Range (min … max):   10.955 s … 11.512 s    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     564.0 ms ±  15.4 ms    [User: 720.6 ms, System: 218.0 ms]
  Range (min … max):   546.3 ms … 573.5 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Summary
  prettier --write --plugin prettier-plugin-java ran
    1.50 ± 0.04 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
   18.30 ± 0.50 times faster than jolt fmt
   19.84 ± 0.74 times faster than google-java-format --replace
```
