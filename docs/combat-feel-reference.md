# Combat feel: readable impact and recovery

Reference: the two user-supplied game-feel images (eight feedback techniques;
anticipation, swing, contact pause, recovery). They illustrate timing and hierarchy,
not a requirement to apply all effects to every attack.

## Existing behavior

- `basic_attacks.rs`: melee damage resolves after `swing_windup`, checking targets
  at actual contact; ranged attacks have their own class presentation.
- `model_animation.rs`: authored attack and hit clips play through the available
  reaction window, scaled by virtual time exactly once.
- `hit_feedback.rs`: survivor squash, critical spark, elemental death bursts,
  stronger elite/Boss kill shake and hit-stop, hero hurt flash/flinch.
- `audio.rs`: ordinary impact, critical, kill and heavy-kill cues have bounded
  real-time playback and pitch variation.
- Hit-stop is skipped above 2x; changing battle speed during a stop takes priority
  over restoring the old speed.

## First change

Dense hits previously reset the survivor's 120ms squash to its zero-deformation
start on every contact. Continuous hits could prevent a visible peak and full
recovery. Multiple events inserted deferred components in arrival order, allowing
an ordinary hit to overwrite a same-frame critical hit.

Group survivor hits by target and take the strongest strength for the frame.
Once an envelope starts, finish impact, overshoot and recovery; another hit may
raise its strength but does not rewind it. After recovery, a fresh ordinary hit
starts at ordinary strength. Damage, targeting, cooldowns, economy and camera
strength rules stay unchanged. Normal attacks do not gain global shake or flashes.

## Validation status

Full library tests passed: 658 passed, zero failed, one ignored. Strict Clippy
passed. Native NVIDIA Vulkan fixtures at 1x and 8x both completed 180 frames,
including production deformation, exact recovery and speed-preservation assertions.
Logs and representative rendered frames are in
[the verification record](verification/combat-feel-2026-11-05/README.md).

Tests cover a 10ms hit barrage recovering exactly to its base scale, the next
ordinary hit losing prior critical strength, and simultaneous critical/ordinary
hits in both event orders. Existing tests cover elite/Boss hierarchy and the 8x
hit-stop bypass.

The fixed-camera sequence shows a visible impact and recovery under a barrage,
a fresh ordinary reaction, a solo critical and an elite-kill feedback event.
This verifies the production feedback boundary, not projectile collision or
actual elite death handling. The unoptimized capture build is visual/input
evidence; it provides no release performance result.

## Further work is separate

Contact alignment of individual authored weapon clips needs its own recorded
sequence before changing animation timing. More particles, full-screen flashes,
universal slow motion or continuous shake are not part of this change. Future
camera/flash additions should honor reduced-motion and reduced-flash settings.

## Native fixture integration

`src/game/hit_feedback/capture.rs` is gated by debug assertions or the existing
`dev-tools` feature. It supplies deterministic contact events at the production
feedback boundary, not fake screen effects or gameplay damage. Production
`react_to_hits`, punch animation, audio, pooled bursts, shake and hit-stop remain
responsible for all output. A real authored enemy model and the game camera are
used. This fixture deliberately does not claim to validate projectile collision.

`src/bin/capture/hit_feedback.rs` exports `configure(world)` and
`install(app, scenario)`. `capture.rs` routes `hit-feedback` and
`hit-feedback-8x` into those functions under the same configuration gate.

Sequence frames: 10 starts critical + ordinary simultaneous contact; 11–45 feeds
ordinary contacts every render frame; 60 requires both a visible deformation peak
and exact recovery; 90 starts a fresh ordinary contact and checks strength 1.0;
120 gives a solo critical; 150 gives an elite death event; 165 verifies requested
battle speed is preserved. Capture frames 9,10,12,14,18,46,60,90,92,120,122,150,
152,165 using the harness frame-sequence mode. At 8x, punch recovery still uses
real time and the heavy-kill event must not slow virtual time.
