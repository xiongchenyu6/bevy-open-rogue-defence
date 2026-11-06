# Primary-window camera target synchronization

The immutable4b518c2 Linux package built successfully but its actual offline Ubuntu24/X11 smoke failed during F11 fullscreen→windowed restoration: depth1600×900 and color1280×720 attachments caused a fatal validation error. That package is rejected. Raw process status, error log and screenshots are preserved in `before/`; this was not an observation timeout.

The new responsive camera wrote Projection every frame. Bevy refreshes a camera’s cached target size when that projection changes; the same-target damage-number2D camera can wait for a later WindowResized message. Programmatic window restoration can change Window.resolution before that message. The regression uses actual Bevy camera_system to demonstrate split cached target sizes when only3D is dirty, then checks synchronization of both cameras without fabricating a window-resize event. It also covers a portrait aspect change and a settled frame without new projection changes. This reproduces the cache mechanism, not a trace of the original GPU failing frame.

The production updater now writes only a changed vertical span or nonzero pan. One PostUpdate system, before CameraUpdateSystems, marks all cameras targeting the actual primary window when their cached physical target size disagrees with that window. Image and other-window targets are excluded. Texture caches, shader code and fatal render-error handling remain unchanged.

Strict release all-target Clippy exited0. Full release library tests: **728 passed,0 failed,1 ignored**, including the real camera_system split→sync regression. Source/assets were frozen throughout. Actual rebuilt standalone fullscreen and portrait restoration acceptance is **pending separately**; these source tests do not establish GPU or public-input success.

Run `python3 docs/verification/primary-camera-resize-2026-11-06/verify.py` to check archived evidence hashes and outcomes. It does not launch a new graphics test.
