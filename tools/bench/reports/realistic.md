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
  Time (mean ± σ):     806.4 ms ±  11.1 ms    [User: 15200.9 ms, System: 794.5 ms]
  Range (min … max):   795.3 ms … 817.4 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     856.9 ms ±  11.6 ms    [User: 17136.6 ms, System: 1168.8 ms]
  Range (min … max):   848.9 ms … 870.1 ms    3 runs
 
Summary
  jolt fmt ran
    1.06 ± 0.02 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
