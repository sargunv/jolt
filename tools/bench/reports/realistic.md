# realistic

Spring Framework Java sources.

## Metadata

| Tool        | Version       | Input files | Modified files |
| ----------- | ------------- | ----------: | -------------: |
| jolt        | jolt 0.0.0    |        9136 |           8620 |
| dprint-jolt | dprint 0.54.0 |        9136 |           8702 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):     790.3 ms ±  24.8 ms    [User: 13190.7 ms, System: 782.7 ms]
  Range (min … max):   762.8 ms … 810.7 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     794.5 ms ±  13.0 ms    [User: 15309.2 ms, System: 1164.1 ms]
  Range (min … max):   786.7 ms … 809.5 ms    3 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs.
 
Summary
  jolt fmt ran
    1.01 ± 0.04 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
