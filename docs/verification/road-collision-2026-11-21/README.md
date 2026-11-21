# Road collision and complete-body camera verification — 2026-11-21

The previous active input clamp did not cover paused/inactive heroes or a position
writer running after input, and camera framing protected the root rather than the
whole body. Three regressions were first run against that implementation: all
three failed (see `regressions-before.log`). This reproduces those defensive gaps;
it does not establish the exact cause of the user's original screenshot.

## Runtime changes

- Paving and collision share road geometry. The commander has a 0.6-unit circular
  footprint; swept movement stops at the first edge and spends remaining motion
  sliding along it. Walking and dodge use the same operation, including very
  large displacements. This is static kinematic collision, not a rigid-body engine.
- An unconditional PostUpdate boundary guard corrects every hero root after all
  Update writers, including paused/dead states and invalid imported positions.
  Hidden/dead vertical positions remain unchanged.
- Camera framing runs after this guard and before transform propagation. It keeps
  the complete standing body box inside the safe view, with a margin for maximum
  camera shake. An alive paused hero is reframed on viewport resize or boundary
  recovery; dead hidden heroes do not pull the camera underground.

## Regression acceptance

Tests cover radius against actual paving, huge corner displacements, wall sliding,
step subdivision, invalid coordinates, paused/dead recovery, a late Update writer,
and all eight body-box corners at 320×240, 844×390, 1479×1054 and 390×844.
Maximum positive/negative camera-right and camera-up shake is checked at all
corners. Full suite: **852 passed, 0 failed, 1 ignored** (`tests-after.log`). All three
previously failing regressions now pass.

## Actual renderer acceptance

The native Bevy renderer runs the Stone Warden used in the report at 1479×1054
and 844×390. Each run holds eight keyboard directions and eight touch-pad
directions against the perimeter, invokes twenty actual dodges, and checks every
frame for road bounds, hero visual/root alignment and all eight projected body
corners inside the viewport. Touch input includes a second finger on dodge and
cancellation; the renderer receives actual Bevy TouchInput messages.

The per-size `capture.log` files record assertions and saved frames. These are
native rendered touch simulations, not a physical Android/iOS device. Hosted
WASM browser testing is reported separately; the cache-update fixture from the
previous release is not gameplay verification.

Both native runs passed all 4,170 frames and the final `PASS collision-perimeter`
assertion. Retained screenshots: frame 380 (keyboard edge), 2710 and 2910 (touch
edges), 4110 (last touch diagonal). Original logs also mention frames 180 and
3310; those redundant images were omitted from the committed evidence.

Reviewed phone frames 2910/4110 and desktop frames 380/4110: Stone Warden remains
on paving next to the railing, with head and feet visible. This validates native
renderer behavior at these viewport sizes, not every device's browser compositor.
