# realistic

Spring Framework Java sources.

## Metadata

| Tool               | Version                            | Input files | Modified files |
| ------------------ | ---------------------------------- | ----------: | -------------: |
| jolt               | jolt 0.0.0                         |        9136 |           8620 |
| dprint-jolt        | dprint 0.54.0                      |        9136 |           8702 |
| google-java-format | google-java-format: Version 1.35.0 |        9136 |           9106 |
| prettier-java      | 3.9.4                              |        9136 |           8698 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):     10.292 s ±  0.046 s    [User: 9.422 s, System: 0.362 s]
  Range (min … max):   10.239 s … 10.325 s    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     842.0 ms ±   6.6 ms    [User: 17020.9 ms, System: 1127.3 ms]
  Range (min … max):   834.4 ms … 846.3 ms    3 runs
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     11.526 s ±  0.048 s    [User: 64.160 s, System: 7.508 s]
  Range (min … max):   11.495 s … 11.581 s    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     28.057 s ±  0.098 s    [User: 34.187 s, System: 1.488 s]
  Range (min … max):   27.945 s … 28.117 s    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Summary
  dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format ran
   12.22 ± 0.11 times faster than jolt fmt
   13.69 ± 0.12 times faster than google-java-format --replace
   33.32 ± 0.28 times faster than prettier --write --plugin prettier-plugin-java
```
