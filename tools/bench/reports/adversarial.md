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
  Time (mean ± σ):      14.0 ms ±   1.8 ms    [User: 74.3 ms, System: 18.1 ms]
  Range (min … max):    12.5 ms …  16.0 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      26.6 ms ±   2.2 ms    [User: 77.0 ms, System: 48.1 ms]
  Range (min … max):    25.0 ms …  29.1 ms    3 runs
 
Benchmark 3: google-java-format --replace
  Time (mean ± σ):     199.4 ms ±  11.5 ms    [User: 386.0 ms, System: 167.9 ms]
  Range (min … max):   189.0 ms … 211.7 ms    3 runs
 
Benchmark 4: prettier --write --plugin prettier-plugin-java
  Time (mean ± σ):     765.3 ms ±   4.6 ms    [User: 1096.4 ms, System: 217.2 ms]
  Range (min … max):   760.6 ms … 769.8 ms    3 runs
 
Summary
  jolt fmt ran
    1.90 ± 0.29 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
   14.25 ± 2.00 times faster than google-java-format --replace
   54.70 ± 7.03 times faster than prettier --write --plugin prettier-plugin-java
```
