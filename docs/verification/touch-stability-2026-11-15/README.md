# Touch camera stability — 2026-11-15

Touch compatibility mouse events previously switched ControlScheme to Keyboard,
changing the camera safe rectangle and HUD on taps and joystick release.
Touch now persists until explicit keyboard/controller input. Mouse input still
selects Keyboard from controller mode; all motion messages are drained each frame.

Verification:
- Before fix: running capture failed at frame 3: `compatibility mouse events must not disable touch mode`.
- After fix: 844×390, 390×844 and 1280×720 captures pass stationary camera assertions
  through tap, joystick press, release, and injected compatibility mouse events.
- 830 library tests passed, 0 failed, 1 ignored. Includes moved/ended touch events,
  delayed mouse events and explicit keyboard switching; existing movement tests pass.
- Screenshots: [idle](idle.png), [held](held.png), [released](released.png).

Raw logs: `target/touch-stability/`. This is native rendered input injection,
not a physical phone/browser touch test. Gameplay movement and combat shake remain enabled.
