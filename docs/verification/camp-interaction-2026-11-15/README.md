# Direct camp interaction — 2026-11-15

The right-hand camp rail is removed. Scene-projected hit areas cover each visible
leader and its camp footprint; offscreen camps are not clamped into detached HUD
buttons. Locked camps reveal unlock time on inspection. Ready camps have a small
sword marker and a brief first-availability hint. Hover/controller focus can reveal
context. All entry areas are hidden during another encounter or modal pause.

Scene taps activate on release. Dragging (including returning to the start), touch
cancellation, active movement-pad fingers and HUD overlap do not open encounters.
Non-button status rails, clocks, XP and trial panels also block scene input.

The existing challenge preview now offers Later alongside Fight and Take & leave.
Later preserves availability and rewards; controller Back uses it. Starting a fight
uses the original camp-to-road arrival and native boss combat. Locked leaders sleep;
unlock produces a short awakening; only real completed camps receive a victory flag.

Verification:
- Full library suite: 831 passed, 0 failed, 1 ignored before the final HUD blockers.
- Final full suite: 830 passed; one existing asynchronous asset retry test timed out
  while concurrent render probes were running. The same compiled test passed alone
  (1 passed in 1.24s). No test was disabled or changed to hide the timeout.
- `camp-interaction` rendered input probe: 844×390, 390×844 and 1280×720 all pass
  locked click, out-and-back drag, cancellation, joystick multi-touch suppression,
  actual scene tap, Later/reopen and real boss arrival/landing (156 frames each).
- `adventure-boss-1`: 1000-frame actual choice/combat/reward/cleanup and relic ability
  probe passes; no synthetic victory event used.
- Controller route verified camp Back/Later (frame767) and reopened safe reward
  (frame835). Probe continued to frame1300; this is not a claim of full-route completion.
  The initial Chinese run stopped at an unrelated English-only tooltip assertion;
  the route was rerun in English as required by that fixture.

Screenshots: [sleeping phone camps](phone-sleeping.png),
[phone preview](phone-preview.png), [portrait preview](portrait-preview.png),
[desktop ready camps](desktop-ready.png), [phone battle](phone-battle.png).

Raw logs and frames: `target/camp-interaction/`. Input is injected through actual
Bevy input/UI paths in the native renderer, not physical phone/controller hardware.
The rendering test deliberately advances unlock time; it does not claim earned progression.

Victory artwork was corrected after screenshot review: the existing hanging cloth
now faces the battle camera and is suspended on a planted pole. A second actual
1000-frame adventure run passed and [the resulting victory marker](victory.png)
was visually inspected. This is a combat-earned completed camp, not a forced mask.

Follow-up: the complete compiled logic suite was rerun serially without concurrent
render probes: **831 passed, 0 failed, 1 ignored** in 52.36s
(`target/camp-interaction/tests-serial.log`). The final victory-pole artwork was
separately built and verified by the second 1000-frame rendered combat probe above.

Deployment verified: GitHub Actions run `43681921263` succeeded; the public
`build.json` reports revision `88431a40af261cd4187ce2b170a6908882fa571e`.
The startup page, manifest, installation script and service worker all returned HTTP 200.
