# Numeric HUD separator follow-up — 2026-11-05

The freshforge-r2 failure occurred in read_speed at speed-source.png (20:13), not the prior unlock-source.png (21:51). Both exact screenshots are preserved. General OCR lost LEVEL7 and XP separators; bounded numeric pixel OCR read LEVEL728/168XP. The unchanged unlock control remains recognizable.

A spatially located LEVEL/numeric/XP candidate now requests the existing exact numeric-pixel validation even when OCR loses separators. It does not repair arbitrary text or infer a level/XP partition. Known modal/choice/terminal checks take priority; valid actual clock and speed are still separately required. Regression is included in the59-test suite at../panel-toggle-2026-11-05/tests.log. Full natural forging remains unverified for040ce5c.
