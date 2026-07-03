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
  Time (mean ± σ):      14.8 ms ±   1.1 ms    [User: 58.5 ms, System: 27.6 ms]
  Range (min … max):    14.2 ms …  16.1 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      24.6 ms ±   2.2 ms    [User: 73.6 ms, System: 50.0 ms]
  Range (min … max):    23.1 ms …  27.2 ms    3 runs
 
Summary
  jolt fmt ran
    1.66 ± 0.19 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
