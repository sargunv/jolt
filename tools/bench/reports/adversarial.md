# adversarial

google-java-format formatter test inputs.

## Metadata

| Tool        | Version       | Input files | Modified files |
| ----------- | ------------- | ----------: | -------------: |
| jolt        | jolt 0.0.0    |         207 |            195 |
| dprint-jolt | dprint 0.54.0 |         207 |            195 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):      17.4 ms ±   0.7 ms    [User: 81.1 ms, System: 27.2 ms]
  Range (min … max):    16.7 ms …  18.0 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      30.4 ms ±   1.2 ms    [User: 94.6 ms, System: 50.8 ms]
  Range (min … max):    29.7 ms …  31.8 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Summary
  jolt fmt ran
    1.74 ± 0.10 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
