# adversarial

google-java-format formatter test inputs.

## Metadata

| Tool               | Version                            | Input files | Modified files |
| ------------------ | ---------------------------------- | ----------: | -------------: |
| jolt               | jolt 0.0.0                         |         207 |            195 |
| dprint-jolt        | dprint 0.54.0                      |         207 |            195 |
| google-java-format | google-java-format: Version 1.35.0 |         207 |            154 |
| prettier-java      | 3.9.4                              |         207 |            198 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):      49.5 ms ±   4.8 ms    [User: 43.4 ms, System: 5.0 ms]
  Range (min … max):    46.7 ms …  55.0 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      26.2 ms ±   3.4 ms    [User: 78.0 ms, System: 41.5 ms]
  Range (min … max):    23.8 ms …  30.1 ms    3 runs
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     201.2 ms ±   7.8 ms    [User: 368.1 ms, System: 186.6 ms]
  Range (min … max):   193.6 ms … 209.3 ms    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     778.9 ms ±   2.0 ms    [User: 1120.6 ms, System: 236.4 ms]
  Range (min … max):   776.9 ms … 781.0 ms    3 runs
 
Summary
  dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format ran
    1.89 ± 0.31 times faster than jolt fmt
    7.68 ± 1.05 times faster than google-java-format --replace
   29.73 ± 3.88 times faster than prettier --write --plugin prettier-plugin-java
```
