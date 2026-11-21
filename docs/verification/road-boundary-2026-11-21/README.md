# Road boundary and screen visibility re-audit — 2026-11-21

## Finding

The reported screenshot shows the hero outside the green camp, beyond the road.
The current production source at `c685359` **did not reproduce that escape** in
this native renderer audit. This is verification evidence, not a claim that a
new movement bug was fixed or proof of which version the reporting browser ran.
The public `build.json` reported `c685359ac5aee3dc8cf4a9747e846b67e25b7ac6` during
investigation; an already-open client can have a different running engine.

No production movement or camera changes were made for this audit. The probe was
expanded to cover the missing horizontal/diagonal directions and actual projected
body visibility, rather than checking only the road width.

## Actual input and renderer checks

`road-boundary` ran 2,550 simulation updates at 30 Hz for each viewport:

- 1479×1054, approximating the supplied desktop screenshot aspect ratio.
- 844×390, landscape phone dimensions.

The existing first 920 frames hold keyboard Up/Down, perform two edge dodges,
then hold the actual touch movement pad in both directions with two additional
second-finger dodges and cancel the touch. The extension holds all eight screen
input directions for 190 frames each and triggers a real Shift dodge in each.
Every dodge must start its actual cooldown.

Every frame checks `abs(x) <= 10`, `abs(z) <= 4.4`, no displacement between the
hero root and its `HeroVisual` global position, and actual camera projection of
the feet/body/head at heights 0.1/1.2/2.4 inside the viewport. Each run passed all
12 dodge checks and all eight direction checks, and exited successfully.

Four retained screenshots were visually inspected. At the green-camp side and
the rear road corner, the hero's feet remain on paving and the model stays in
frame. Logs include the intermediate screenshot writes; only the two endpoint
images per viewport are retained to avoid committing redundant large images.
This does not test physical Android/iOS hardware, browser version activation,
or all possible combat states. The second phase uses keyboard events at both
viewport sizes; the first phase covers actual touch messages.

## Code/asset audit

Production movement and dodge share the road clamp. Respawn/run-reset positions
are on the road. Ember rebirth restores the prior position. Void pull changes
**enemy** transforms, not the hero. Stone Warden's GLB root translation is zero
through Idle/Walk/Attack/Hit, with only small hip animation offsets; no large root
motion explains the supplied screenshot.

## Reproduce

```sh
nix develop --command cargo build --bin capture
nix develop --command env \
  VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json \
  ROGUE_COMMANDER_LANG=en ROGUE_CAPTURE_SAVE_FRAMES=180,2510 \
  target/debug/capture frames /tmp/boundary-reaudit 2550 road-boundary 0xA11CE 1479x1054
```

Repeat with `844x390`. Acceptance line: `PASS full-boundary`.
