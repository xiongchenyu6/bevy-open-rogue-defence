# Authored Hit recovery under continuous damage

Repeated health loss restarted the skeletal Hit clip on every frame. The earlier
scale-punch fix did not fix this separate authored-animation path. Production now
finishes an active Hit reaction before accepting another damage-triggered Hit.
A real attack may still select an attack pose; existing hostile-attack protection,
gameplay damage, movement, stun/freeze and control rules retain their behavior.

The native baseline reproduces the old behavior: all 36 observations during the
continuous HP-loss window report Hit elapsed `.033333331s`. After the change,
the same 36 observations advance through `.0667`, `.1000`, `.1333`, `.1667`,
`.2000`, `.2333` and `.2667s`, then accept a fresh reaction. The 8x regression
also completes and preserves the requested virtual speed. Its coarse simulation
step naturally completes a Hit in one render step; it is not evidence that the
old restart defect was visible at 8x.

`before-restarted.png`, `after-progress.png`, `after-late-reaction.png` and
`after-recovered.png` are actual NVIDIA Vulkan frames personally reviewed by the
primary agent. Use the synchronized playback logs for exact times: the screenshot
request pipeline is asynchronous. These unoptimized fixtures are not performance
benchmarks or release packages.

The opt-in `ROGUE_CAPTURE_AUTHORED_HITS=1` diagnostic decreases the isolated
target's actual HP by one on frames 10–45. Production health detection, clip
selection and Bevy playback generate all skeletal output. Contact events still
drive the existing visual/audio feedback fixture. This is injected presentation
stimulus, not proof of projectile collisions, real damage calculation or earned
progression. Default capture behavior is unchanged.

Native runs, each 180 frames at 1280x720 with seed `0xA11CE`:

```sh
ROGUE_CAPTURE_AUTHORED_HITS=1 ROGUE_CAPTURE_SAVE_FRAMES=10,12,14,18,30,40,46,60 \
  nix develop --command target/market-visual/debug/capture frames \
  target/verification/authored-hit-2026-11-05/after \
  180 hit-feedback 0xA11CE 1280x720
```

Use isolated campaign/tutorial/audio paths and the appropriate Vulkan ICD.
Repeat with `hit-feedback-8x`. The archived baseline is gameplay from `60f9f22`
with only the diagnostic fixture and new regression test added; its expected
restart assertion used `ROGUE_CAPTURE_EXPECT_HIT_RESTART=1`. The current source
correctly fails that old-behavior expectation.

Full library suite: **659 passed, zero failed, one ignored**. The new ECS test
uses actual hero health loss to verify retained playback position, return to Idle
and a fresh reaction after recovery. Existing enemy attack, pause/control and
pooled-animation tests also pass. Strict Clippy passed. Source/binary hashes and
the measured playback values are in `evidence.json`.
