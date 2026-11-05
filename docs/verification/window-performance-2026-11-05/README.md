# Native-window performance workload

These are optimized production-app runs with an injected dense combat fixture,
including real skill, projectile, damage and animation systems. The probe uses
the ordinary window render/present path and performs no screenshot readback.
The measured quantity is wall-clock time between application present calls;
GPU execution timestamps and compositor-displayed frames are outside its scope.

## Actual desktop result

Hardware: AMD Ryzen 7 5800H (8 cores/16 threads), NVIDIA RTX 3070 Laptop GPU,
driver 615.78.08, Vulkan, NixOS Linux 7.3.0. The actual Wayland surface is
**1920×1080**, despite the requested 1280×720 logical window. Each case has
10 seconds of warmup after model instantiation and 30 seconds of sampling.
Desktop/browser services remained running; no project build or capture workload
ran during sampling. One final run per desktop case is recorded here.

| Case | Intervals | P50 ms | P95 ms | P99 ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Actual Wayland, FIFO, 1x | 3203 | 9.07 | 11.25 | 14.25 | 17.21 |
| Actual Wayland, FIFO, 32x | 3194 | 9.03 | 11.47 | 14.40 | 17.02 |

The raw JSON/CSV files are authoritative for exact counts and values. These
results establish a finite workload baseline on this machine. They do not
certify a complete long campaign, every evolved build, other hardware or Steam
Deck. The local Nix-linked optimized main binary is also not a portable Steam
Runtime release candidate; the previous independent candidate remains separate.

## Workload and measurement checks

- 180 tagged stationary targets use all 12 imported enemy models, with `1e7` HP.
  Their actual `WorldInstance` objects must be ready before warmup. Required
  model/font/atlas handles are all loaded (29/29 in these runs).
- Hero position is fixed at `(-2,0,0)` so ordinary attacks can reach targets.
  Six skills remain rank 3; gameplay begins at virtual elapsed 720s. Hero/gate
  health is protected. Spawning, Autoplay, automatic trials/bond draws and
  recruitment milestones cannot open extra dialogs or alter the target count.
- Every run requires real HP loss, positive-lifetime projectiles and production
  skill casts during measurement. Rank invariants, 180 valid targets, speed,
  pause and terminal state are checked. Passive skills and a capped summon can
  legitimately report zero new casts. The 32x cast totals visibly increase.
- `verify_perf_probe.py` recomputes nearest-rank percentiles from every raw
  interval and checks counters/workload snapshots. The negative checks reject
  wrong percentiles, absent damage, incorrect speed and incomplete assets.
- The probe's release unit test passes, as does strict all-target Clippy. The
  production gameplay library is unchanged from `358ff50` (659 passing tests).

This remains injected presentation/gameplay load rather than earned campaign
progression. Audio is muted, mesh counts include pooled/hidden components and
the projectile peak comes from 1Hz snapshots. These distinctions matter when
interpreting the files.

## Virtual-display controls and CPU sample

| Xvfb 1280×720 case | P50 ms | P95 ms | P99 ms |
| --- | ---: | ---: | ---: |
| FIFO, 1x | 30.62 | 36.22 | 40.59 |
| FIFO, 32x | 30.72 | 36.00 | 38.03 |
| Immediate, 1x | 30.55 | 35.64 | 39.84 |

Disabling FIFO did not materially change the virtual-display result. The real
Wayland result is substantially different, with both display path and physical
surface dimensions changing. Therefore the Xvfb measurements cannot determine
actual desktop playability or a unique cause of the difference.

An instrumented Xvfb CPU profile was enabled only after `PERF_SAMPLE_BEGIN`, using
`perf` user-only callchains at 99Hz. It collected 2295 samples with zero lost
samples. Largest self user-CPU groups were transform propagation (17.30%),
animation-target evaluation (3.09%), memory movement (3.05%) and skin extraction
(2.00%). These are shares of sampled busy CPU time, not percentages of wall
frame time or guaranteed savings. The raw 19MB profile stays at the path/hash in
`cpu-profile.json`; derived symbols and capture identity are archived there.
The profile predates the probe-only presentation-mode switch and uses the same
gameplay source. Its capture binary identity is retained separately.

No model animation, effect density, graphic quality or gameplay rule was reduced
to obtain the desktop results.

## Reproduction

Build the standard repository release profile:

```sh
nix develop --command cargo build --locked --release --bin perf_probe -j2
nix develop --command cargo test --locked --release --bin perf_probe -j2
nix develop --command cargo clippy --locked --all-targets -- -D warnings
```

Run `perf_probe SPEED WARMUP_SECONDS SAMPLE_SECONDS OUTPUT_PREFIX` with explicit
`BEVY_ASSET_ROOT`, isolated config/cache/campaign/audio/tutorial/detail paths,
seed `0xA11CE`, and recorded purpose/build/concurrent-workload environment fields.
Keep the real compositor's runtime socket for Wayland. The recorded runners
(`run-wayland-1x.sh`, `run-wayland-32x.sh`, `run-x11.sh`) preserve the commands and
machine-specific display/tool paths used here; adapt those paths for another
session. X11 used Xvfb 21.1.24 and Openbox 3.6.1 from the locked flake input.

```sh
python tools/desktop/verify_perf_probe.py \
  docs/verification/window-performance-2026-11-05/desktop-32x \
  --minimum-seconds 30 --surface 1920x1080
```

`evidence.json` preserves source and binary hashes, adapter strings and all five
final case summaries. The raw per-present CSV, 1Hz population CSV and completion
log accompany each case. `run-cpu-profile.sh` records the user-only CPU sampling
method; it collects diagnostic rather than baseline timing.
