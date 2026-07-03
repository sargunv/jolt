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
  Time (mean ± σ):     844.5 ms ±  39.7 ms    [User: 17264.7 ms, System: 915.9 ms]
  Range (min … max):   815.8 ms … 889.8 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     891.2 ms ±  32.4 ms    [User: 17788.0 ms, System: 1189.1 ms]
  Range (min … max):   857.8 ms … 922.4 ms    3 runs
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     11.044 s ±  0.139 s    [User: 64.168 s, System: 7.376 s]
  Range (min … max):   10.894 s … 11.170 s    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     28.223 s ±  0.213 s    [User: 34.658 s, System: 1.378 s]
  Range (min … max):   27.992 s … 28.411 s    3 runs
 
Summary
  jolt fmt ran
    1.06 ± 0.06 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
   13.08 ± 0.64 times faster than google-java-format --replace
   33.42 ± 1.59 times faster than prettier --write --plugin prettier-plugin-java
```
