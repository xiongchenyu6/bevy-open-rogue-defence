# Continuous-hit recovery verification

The user supplied references about anticipation, contact, recovery and feedback
hierarchy. This pass fixes one concrete timing defect: successive hits rewound
the 120ms reaction before it could complete, and deferred same-frame ordinary
hits could replace a critical reaction.

Native NVIDIA RTX 3070 Vulkan runs completed 180 frames each for `hit-feedback`
and `hit-feedback-8x`. Both logs contain the dense-hit peak/recovery, fresh normal
strength and requested battle speed assertions, followed by normal capture
completion. The fixture injects deterministic contact events into production
feedback systems; it does not deal real damage or prove projectile collision.
The elite-kill event exercises feedback, without pretending the target died.

Representative actual rendered frames:

- `critical-impact.png`: compressed survivor during the initial combined hit.
- `recovered.png`: restored silhouette after the barrage.
- `ordinary-impact.png`: subsequent ordinary reaction.
- `recovered-8x.png`: restored silhouette in the 8x run.

The screenshot pipeline is asynchronous, so screenshot request numbers are not
an exact synchronization point for a probe's Update counter. The accompanying
logs record actual production scales: initial critical `(1.795, 1.132, 1.795)`,
ordinary `(1.684, 1.270, 1.684)`, restored `(1.5, 1.5, 1.5)`.
The primary agent viewed impact, intermediate and recovered full frames.

Reproduction uses the ordinary capture CLI with isolated save paths and NVIDIA
Vulkan, for example:

```sh
ROGUE_CAPTURE_SAVE_FRAMES=9,10,12,14,18,46,60,90,92,120,122,150,152,165 \
  nix develop --command target/market-visual/debug/capture frames \
  target/verification/combat-feel-2026-11-05/hit-feedback \
  180 hit-feedback 0xA11CE 1280x720
```

Repeat with `hit-feedback-8x`. The binary was built with dependency and project
dev opt-level 0, debug info 0, one Cargo job. This is visual and behavior
verification, not a performance benchmark or release package. Full library
suite: 658 passed, zero failed, one ignored; strict Clippy passed.
