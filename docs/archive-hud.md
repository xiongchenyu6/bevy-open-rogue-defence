# Archive HUD placement — 2026-10-09

The real fifth-chapter screenshot `verification/packaged-chapter5-2026-10-09-r2/run-146.png` showed archive health text overflowing into the speed toolbar. Countdown, echo health and hero health now use a separate centered block with one row per value and a dark translucent background. Desktop width is 200 px/top 80 px; compact width is 128 px/top 118 px. The normal top-right timer is empty during archive combat and restores afterward. Controls are unchanged.

Final evidence: `verification/archive-hud-2026-10-09/final-contrast/`. The combined check passed 640 library tests (one ignored), strict all-target Clippy and main/capture build. Four sequential native runs (English and Chinese, 1280×720 and 667×375), 260 frames each, saved before/archive/after screenshots (12 PNGs, individually inspected). The final native unit `rogue-archive-hud-native-oct9-r5.service` exited 0.

Actual computed archive rectangles: desktop `(540,80)..(740,138)`; compact `(270,118)..(398,176)`. The compact left HUD ended at x267 and speed controls ended at y112. The fixture asserts actual archive rectangles do not intersect either root, three separate text rows remain, and archive visibility ends. The screenshots also show readable text and restoration of the ordinary countdown.

The original unbacked 200 px version remains in the evidence root for comparison. Adding contrast initially covered the compact energy-bar edge; that rejected 12-image run is retained in `rejected-contrast-overlap/`. Reducing only the compact width eliminated this regression. A stopped intermediate build was incomplete and is not used as final evidence.

This is a presentation fixture: it advances the campaign clock to archive combat, allows the real echo to spawn, pauses the encounter to retain it, then clears the archive UI flag to inspect restoration. It proves layout behavior, not an earned clear, difficulty, percentage evolution or full UI polish. Transient notifications were not exercised simultaneously with archive progress.
