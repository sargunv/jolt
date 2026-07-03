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
  Time (mean ± σ):     797.2 ms ±  32.2 ms    [User: 14784.1 ms, System: 821.4 ms]
  Range (min … max):   766.9 ms … 831.1 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     840.0 ms ±  20.8 ms    [User: 16781.8 ms, System: 1160.5 ms]
  Range (min … max):   826.9 ms … 864.0 ms    3 runs
 
  Warning: The first benchmarking run for this command was significantly slower than the rest (864.0 ms). This could be caused by (filesystem) caches that were not filled until after the first run. You are already using both the '--warmup' option as well as the '--prepare' option. Consider re-running the benchmark on a quiet system. Maybe it was a random outlier. Alternatively, consider increasing the warmup count.
 
Summary
  jolt fmt ran
    1.05 ± 0.05 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
