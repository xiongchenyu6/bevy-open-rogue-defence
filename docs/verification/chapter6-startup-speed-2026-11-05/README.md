# Startup speed proof uses public toolbar pixels

Chapter-six equipment R4 stopped before base attributes: whole-screen OCR did not
return an isolated `1×` line. Root inspected `actual-1x.png`; the toolbar clearly
shows1×. This was a harness interruption, not a terminal battle result.

The common smoke helper now reads the verified1280×720 speed-button region
(38×28 at1029,32), enlarges it and requires one complete speed label. It accepts
only1/2/4/8/16/32 followed by x/×; missing units, unsupported numbers and mixed
labels fail. Both campaign attribute sampling and actual requested-speed
confirmation use this public-pixel helper. The attribute stat, hero/level, save,
price and result checks are retained; no game state is read or changed.

Docker tests:19 passed, including OCR of this exact native failure frame,
strict speed parsing, existing campaign stat/seed checks and equipment-policy
regressions. This is fixed-geometry test automation, not a new game input path
or a cross-resolution claim. Live R5 remains a separate pending route.
