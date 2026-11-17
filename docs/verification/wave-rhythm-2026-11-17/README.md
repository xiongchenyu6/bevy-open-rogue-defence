# Grouped wave cadence — actual renderer verification

The capture uses the real `spawn_wave` system and authored game assets, a fixed
seed (`0xA11CE`), and a 30 Hz simulation step. Hero attacks are disabled and hero/
gate health is raised to keep arrivals observable. These are scheduling/rendering
probes, not player win-rate or difficulty claims.

## Accepted captures

- `wave-1/` — 1280×720, the opening minute. Actual actor totals after the three
  arrivals are **4 → 12 → 18** (groups of **4, 8, 6**).
- `wave-4/` — 844×390. Actual totals are **8 → 22 → 32** (groups of **8, 14, 10**).
- `wave-4-desktop/` — repeat at 1280×720; reviewed main arrival `frame00630.png` clearly shows a mixed group entering together. Same **8 → 22 → 32** actor totals and all assertions pass.
- All runs assert three real warnings and three announced group arrivals, freeze
  a warning during a deliberate pause, and verify later fourth-wave arrivals
  include real elite enemies. Every selected screenshot was written successfully.
- Screenshots reviewed: opening warning (`wave-1/frames/frame00090.png`), first
  group (`frame00150.png`), main group (`frame00630.png`), and fourth-wave main/
  closing groups (`wave-4/frames/frame00630.png`, `frame01350.png`). The entrance
  visibly changes to red before the arrival; multiple modeled enemies appear
  together rather than one per interval.

The mobile screenshot also exposes an existing layout limitation: the top-right
HUD partly overlaps the portal. This capture does not claim to fix that separate
layout issue. Physical mobile devices are not covered by an offscreen render.

## Reproduce

Run inside the repository's Nix development shell (required for the Vulkan loader):

```sh
nix develop --command cargo build --bin capture
nix develop --command env \
  VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json \
  ROGUE_COMMANDER_LANG=en \
  ROGUE_CAPTURE_SAVE_FRAMES=90,150,510,630,1230,1350,1789 \
  target/debug/capture frames /tmp/rogue-wave-proof 1795 raid-pressure-1 0xA11CE 1280x720
```

Use `raid-pressure-4` and `844x390` for the second probe. The 1795 updates cover
one minute including the deliberate pause; PNG selection only reduces disk output,
not simulation work. `timings.csv` is serialized simulation/render-wait telemetry,
not a mobile FPS benchmark. The logs identify the actual NVIDIA Vulkan adapter.

## Tests and publication

- Actual schedule tests: 5 passed (`schedule-tests.txt`).
- Full native library suite: 844 passed, 0 failed, 1 ignored (`full-tests.log`),
  including real ECS group delivery at 1/2/4/8/16/32x speed.
- Pages run 44061917790 succeeded for `5abbfa07f53f7cc66520f412ead184833c34d6e7`.
  Public build and manifest revisions match; downloaded compressed WASM passed
  SHA-256 verification (`public-release.json`).
