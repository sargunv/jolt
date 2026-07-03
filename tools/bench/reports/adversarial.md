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
  Time (mean ± σ):      11.6 ms ±   1.2 ms    [User: 54.9 ms, System: 22.1 ms]
  Range (min … max):    10.3 ms …  12.7 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      23.5 ms ±   0.4 ms    [User: 72.8 ms, System: 42.9 ms]
  Range (min … max):    23.3 ms …  24.0 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Summary
  jolt fmt ran
    2.04 ± 0.21 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
