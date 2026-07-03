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
  Time (mean ± σ):     805.5 ms ±  37.5 ms    [User: 13869.8 ms, System: 760.4 ms]
  Range (min … max):   766.3 ms … 841.2 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     796.1 ms ±  22.4 ms    [User: 15611.4 ms, System: 1114.1 ms]
  Range (min … max):   780.7 ms … 821.8 ms    3 runs
 
Summary
  dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format ran
    1.01 ± 0.06 times faster than jolt fmt
```
