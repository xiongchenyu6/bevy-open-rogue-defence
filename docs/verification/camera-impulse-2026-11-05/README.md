# Native camera impact regression

The first shake fix (`48797f1`) aggregates simultaneous contacts rather than
summing every projectile. Native verification then exposed another issue:
`apply_screen_shake` decayed trauma before applying the camera offset. The normal
critical impulse of 0.07 was consumed by the 0.08 decay at a 30 Hz probe step.
Both critical bursts at frames 10 and 120 had exactly zero camera offset.

Production now applies the impulse before its normal decay. The same native
fixture shows nonzero camera offsets at those contacts, returns to zero after
recovery, and retains the requested battle speed after the elite-kill pause.
The strength constants, damage, attack cadence and hit-stop rules are unchanged.
This restores a deliberately small impulse; nonzero is not evidence of a large
or subjectively satisfying camera effect.

## Evidence

- Real NVIDIA Vulkan native window, 1280×720 requested, 180 updates per run,
  30 Hz deterministic simulation; 1× and 8× before and after, each with 12 saved
  native frames. All four runs finished with explicit completion and no panic.
- The debug-only `ROGUE_CAPTURE_SHAKE_VOLLEY=1` feeds 100 critical contacts on
  frames 10 and 120. The production hit feedback, impact pool, time controller
  and camera systems process them. The fixture injects contact events, not real
  projectile damage; the elite-kill event does not kill its diagnostic survivor.
- `comparison.json` contains actual camera offsets/trauma. `compare.py` rejects
  missing completion, zero post-fix critical impulses, excess trauma and failure
  to recover. Run it against the target evidence directory (logs retain original
  capture names).
- Optimized full library suite: **654 passed, 0 failed, 1 ignored**. The new ECS
  test advances real time by 100 ms, requires a nonzero first camera impulse,
  and checks exact return to the original camera translation next update.
- The primary agent inspected native before/after critical and elite-kill images.
  Async screenshots are illustrative; synchronized transform logs establish the
  exact contact timing. Still images cannot demonstrate subjective motion feel.
- The earlier same-frame aggregation regression is included in the passing suite.
  Production retains the existing 2× hit-stop restore test and 8× bypass test.

## Reproduction

Build the debug capture with `profile.dev.opt-level=0`,
`profile.dev.package."*".opt-level=0`, `profile.dev.debug=0`. Use isolated
campaign/tutorial/audio/config paths, `BEVY_ASSET_ROOT` set to the repository and
an actual native display plus Vulkan driver. Run:

```sh
ROGUE_CAPTURE_SHAKE_VOLLEY=1 \
ROGUE_CAPTURE_SAVE_FRAMES=8,10,12,18,60,118,120,122,148,150,152,165 \
  nix develop --command target/market-visual/debug/capture frames \
  OUTPUT 180 hit-feedback 0xA11CE 1280x720
```

Repeat with `hit-feedback-8x`. The before build uses production `48797f1` with
only the new fixture stimulus and trace; the post-fix fixture additionally asserts
that the first critical impulse reaches the camera. Hashes identify current
source and capture binary. These unoptimized capture runs are not performance
benchmarks, portable package acceptance or complete combat-feel acceptance.
